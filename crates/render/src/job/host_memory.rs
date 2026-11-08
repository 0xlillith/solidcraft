//! A shared native RAM allowance for all image hosts. The OS capacity snapshot
//! stays fixed while leases exist, so resident buffers are not charged twice.
use serde::Serialize;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum HostMemorySource {
    SystemAvailable,
    SystemAndCgroup,
    ConservativeFallback,
}

/// Point-in-time admission against the process-wide native render allowance.
/// Required bytes include this request's retained metadata and capture credit.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct HostMemoryAdmission {
    /// This request's total debit, including any already-held capture credit.
    pub required_bytes: u64,
    /// Other held leases in the shared native render ledger.
    pub admitted_bytes: u64,
    /// Capacity available to this request before subtracting required_bytes.
    pub available_bytes: u64,
    /// The idle-time RAM snapshot, bounded by the finite software ceiling.
    pub capacity_bytes: u64,
    pub source: HostMemorySource,
}

#[cfg(not(target_arch = "wasm32"))]
mod native {
    use super::{HostMemoryAdmission, HostMemorySource};
    use crate::job::{ErrorCode, JobError, NATIVE_CPU_POOL_BYTES, Result};
    use std::sync::{Arc, Mutex, OnceLock};

    const FALLBACK_BYTES: u64 = 512 * 1024 * 1024;

    #[derive(Clone, Copy)]
    pub(crate) struct MemorySnapshot {
        pub(crate) capacity_bytes: u64,
        pub(crate) source: HostMemorySource,
    }
    impl MemorySnapshot {
        pub(crate) fn from_probe(total: u64, available: u64, cgroup: Option<u64>) -> Self {
            if total == 0 {
                return Self { capacity_bytes: FALLBACK_BYTES.min(cgroup.unwrap_or(u64::MAX)), source: HostMemorySource::ConservativeFallback };
            }
            let capacity_bytes = available.min(total).min(cgroup.unwrap_or(u64::MAX)).min(NATIVE_CPU_POOL_BYTES);
            Self { capacity_bytes, source: if cgroup.is_some() { HostMemorySource::SystemAndCgroup } else { HostMemorySource::SystemAvailable } }
        }
    }

    fn system_snapshot() -> MemorySnapshot {
        let Ok(mut system) = sysinfo::System::new() else {
            return MemorySnapshot::from_probe(0, 0, None);
        };
        system.refresh_memory_specifics(sysinfo::MemoryRefreshKind::nothing().with_ram());
        // Linux service/container ceilings can be below the root cgroup's limit.
        // Refresh only our process header, without task, CPU, disk or environment data.
        #[cfg(target_os = "linux")]
        let cgroup = {
            let pid = sysinfo::Pid::from_u32(std::process::id());
            system.refresh_processes_specifics(sysinfo::ProcessesToUpdate::Some(&[pid]), false, sysinfo::ProcessRefreshKind::nothing());
            system.process(pid).and_then(sysinfo::Process::cgroup_limits).or_else(|| system.cgroup_limits())
        };
        #[cfg(not(target_os = "linux"))]
        let cgroup = system.cgroup_limits();
        MemorySnapshot::from_probe(system.total_memory(), system.available_memory(), cgroup.map(|limits| limits.free_memory.min(limits.total_memory)))
    }

    struct State {
        snapshot: MemorySnapshot,
        used: u64,
        invalid: bool,
        #[cfg(test)]
        injected: Option<MemorySnapshot>,
    }
    impl State {
        fn refresh_if_idle(&mut self) {
            if self.used != 0 || self.invalid {
                return;
            }
            #[cfg(test)]
            if let Some(snapshot) = self.injected {
                self.snapshot = snapshot;
                return;
            }
            self.snapshot = system_snapshot();
        }

        fn receipt(&self, required_bytes: u64, credit: u64) -> Result<HostMemoryAdmission> {
            if self.invalid {
                return Err(JobError::new(ErrorCode::Internal, "native render memory ledger is inconsistent"));
            }
            let admitted_bytes = self
                .used
                .checked_sub(credit)
                .ok_or_else(|| JobError::new(ErrorCode::Internal, "native memory credit exceeds the held render leases"))?;
            let capacity_bytes = self.snapshot.capacity_bytes;
            let available_bytes = capacity_bytes.saturating_sub(admitted_bytes);
            if required_bytes > available_bytes {
                let source = match self.snapshot.source {
                    HostMemorySource::SystemAvailable => "system available RAM",
                    HostMemorySource::SystemAndCgroup => "system and cgroup available RAM",
                    HostMemorySource::ConservativeFallback => "512 MiB fallback because the RAM query was unavailable",
                };
                return Err(JobError::new(
                    ErrorCode::LimitExceeded,
                    format!(
                        "native render memory needs {required_bytes} additional bytes; {admitted_bytes} bytes already admitted, \
                         {available_bytes} bytes available from the {capacity_bytes}-byte {source} snapshot"
                    ),
                ));
            }
            Ok(HostMemoryAdmission { required_bytes, admitted_bytes, available_bytes, capacity_bytes, source: self.snapshot.source })
        }
    }

    pub(crate) struct NativeMemoryPool {
        state: Mutex<State>,
    }
    impl NativeMemoryPool {
        fn new() -> Self {
            Self {
                state: Mutex::new(State {
                    snapshot: MemorySnapshot::from_probe(0, 0, None),
                    used: 0,
                    invalid: false,
                    #[cfg(test)]
                    injected: None,
                }),
            }
        }

        pub(crate) fn check(&self, required_bytes: u64, credit: u64) -> Result<HostMemoryAdmission> {
            let mut state = self.state.lock().map_err(|_| JobError::new(ErrorCode::HostUnavailable, "native render memory lock poisoned"))?;
            state.refresh_if_idle();
            state.receipt(required_bytes, credit)
        }

        pub(crate) fn reserve(self: &Arc<Self>, bytes: u64) -> Result<MemoryLease> {
            let mut state = self.state.lock().map_err(|_| JobError::new(ErrorCode::HostUnavailable, "native render memory lock poisoned"))?;
            state.refresh_if_idle();
            state.receipt(bytes, 0)?;
            state.used =
                state.used.checked_add(bytes).ok_or_else(|| JobError::new(ErrorCode::LimitExceeded, "native memory reservation overflow"))?;
            Ok(MemoryLease { pool: self.clone(), bytes })
        }

        fn release(&self, bytes: u64) {
            if bytes == 0 {
                return;
            }
            let mut state = self.state.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
            if let Some(used) = state.used.checked_sub(bytes) {
                state.used = used;
            } else {
                // An inconsistent ledger must never create more credit.
                state.snapshot.capacity_bytes = 0;
                state.invalid = true;
            }
        }

        #[cfg(test)]
        pub(crate) fn testing(capacity_bytes: u64) -> Arc<Self> {
            let snapshot = MemorySnapshot { capacity_bytes, source: HostMemorySource::SystemAvailable };
            Arc::new(Self { state: Mutex::new(State { snapshot, used: 0, invalid: false, injected: Some(snapshot) }) })
        }

        #[cfg(test)]
        pub(crate) fn set_test_capacity(&self, capacity_bytes: u64) {
            self.state.lock().unwrap().injected = Some(MemorySnapshot { capacity_bytes, source: HostMemorySource::SystemAvailable });
        }
    }

    pub(crate) struct MemoryLease {
        pool: Arc<NativeMemoryPool>,
        bytes: u64,
    }
    impl MemoryLease {
        pub(crate) fn split(&mut self, bytes: u64) -> Result<Self> {
            if bytes > self.bytes {
                return Err(JobError::new(ErrorCode::LimitExceeded, "native memory split exceeds its lease"));
            }
            self.bytes -= bytes;
            Ok(Self { pool: self.pool.clone(), bytes })
        }

        pub(crate) fn merge_from(&mut self, other: &mut Self) -> Result<()> {
            if !Arc::ptr_eq(&self.pool, &other.pool) {
                return Err(JobError::new(ErrorCode::WrongOwner, "native memory leases belong to different pools"));
            }
            self.bytes =
                self.bytes.checked_add(other.bytes).ok_or_else(|| JobError::new(ErrorCode::LimitExceeded, "native memory merge overflow"))?;
            other.bytes = 0;
            Ok(())
        }

        pub(crate) fn shrink(&mut self, bytes: u64) -> Result<()> {
            let released =
                self.bytes.checked_sub(bytes).ok_or_else(|| JobError::new(ErrorCode::LimitExceeded, "native memory shrink exceeds its lease"))?;
            self.pool.release(released);
            self.bytes = bytes;
            Ok(())
        }
    }
    impl Drop for MemoryLease {
        fn drop(&mut self) {
            self.pool.release(self.bytes);
        }
    }

    pub(crate) fn shared_native_memory() -> Arc<NativeMemoryPool> {
        static MEMORY: OnceLock<Arc<NativeMemoryPool>> = OnceLock::new();
        MEMORY.get_or_init(|| Arc::new(NativeMemoryPool::new())).clone()
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
pub(crate) use native::MemorySnapshot;
#[cfg(not(target_arch = "wasm32"))]
pub(crate) use native::{MemoryLease, NativeMemoryPool, shared_native_memory};
