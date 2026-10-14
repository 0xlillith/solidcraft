//! File dialogs that never block the event loop (#1088).
//!
//! A command that needs a file asks with [`SolveApp::pick_open`] or [`SolveApp::pick_save`] and
//! hands over what to do with the answer. The host calls [`SolveApp::poll_file_dialog`] every
//! frame: it shows the queued dialog (with the window as parent) and the action continues on a
//! later frame. A dialog run to completion inside the frame ran AppKit's modal loop inside
//! winit's event handler, which aborts the app on macOS when a resize event arrives, and froze
//! the window on Linux and Windows.
//!
//! Adapted from PhotoCraft (storytold/photocraft, Apache-2.0), `crates/ui-egui/src/file_dialog.rs`.

use std::sync::Arc;
use std::task::{Context as TaskContext, Poll, Wake, Waker};

use crate::SolveApp;

/// What a file dialog asks the user for.
pub enum FileDialogRequest {
    /// A file to read (the host's own filters, as File > Open uses today).
    Open,
    /// Where to write: starts with `name`, offers `extensions`.
    Save { name: String, extensions: Vec<String> },
}

/// The user's answer, later: the chosen path, or `None` when cancelled.
pub type FileDialogAnswer = std::pin::Pin<Box<dyn std::future::Future<Output = Option<String>>>>;

/// The file dialog in progress.
pub(crate) struct Pending {
    /// The design that asked: the answer applies to it, wherever the user has gone since.
    tab: u64,
    /// Not shown yet: the host shows it at the end of the frame, which has the window.
    request: Option<FileDialogRequest>,
    answer: Option<FileDialogAnswer>,
    then: Box<dyn FnOnce(&mut SolveApp, Option<String>)>,
}

/// The status error shown when a request meets a dialog that is already shown.
pub(crate) const ALREADY_OPEN: &str = "A file dialog is already open";

/// Wakes the app: a dialog answered from the platform's own thread asks for a new frame.
struct Repaint(egui::Context);

impl Wake for Repaint {
    fn wake(self: Arc<Self>) {
        self.0.request_repaint();
    }
}

impl SolveApp {
    /// For callers that chain with [`Self::after_file_dialog`]: refuse, with the status error,
    /// when a dialog is open (they would attach to someone else's). True when refused.
    pub(crate) fn refuse_if_file_dialog_open(&mut self) -> bool {
        let open = self.file_dialog_open();
        if open {
            self.set_status(ALREADY_OPEN, true);
        }
        open
    }

    /// Whether a file dialog is shown or waiting to be shown.
    pub fn file_dialog_open(&self) -> bool {
        self.file_dialog.is_some()
    }

    /// Ask for a file to read; `then` gets its path on a later frame (not on Cancel).
    pub(crate) fn pick_open(&mut self, then: impl FnOnce(&mut SolveApp, String) + 'static) {
        self.queue_file_dialog(FileDialogRequest::Open, then);
    }

    /// Ask where to write; `then` gets the path on a later frame (not on Cancel).
    pub(crate) fn pick_save(&mut self, name: &str, extensions: &[&str], then: impl FnOnce(&mut SolveApp, String) + 'static) {
        let request = FileDialogRequest::Save { name: name.into(), extensions: extensions.iter().map(|e| (*e).into()).collect() };
        self.queue_file_dialog(request, then);
    }

    fn queue_file_dialog(&mut self, request: FileDialogRequest, then: impl FnOnce(&mut SolveApp, String) + 'static) {
        if self.file_dialog.as_ref().is_some_and(|p| p.request.is_none()) {
            self.set_status(ALREADY_OPEN, true);
            return;
        }
        let then = Box::new(move |app: &mut SolveApp, path: Option<String>| {
            if let Some(path) = path {
                then(app, path);
            }
        });
        self.file_dialog = Some(Pending { tab: self.docs.active_id(), request: Some(request), answer: None, then });
    }

    /// Run `next` after the pending dialog's `then` (also on Cancel). False when none is pending.
    pub(crate) fn after_file_dialog(&mut self, next: impl FnOnce(&mut SolveApp) + 'static) -> bool {
        let Some(p) = self.file_dialog.as_mut() else { return false };
        let then = std::mem::replace(&mut p.then, Box::new(|_, _| {}));
        p.then = Box::new(move |app, path| {
            then(app, path);
            next(app);
        });
        true
    }

    /// Called by the host every frame: shows a queued dialog through `show`, and continues the
    /// action that asked once the answer is in.
    pub fn poll_file_dialog(&mut self, ctx: &egui::Context, show: impl FnOnce(FileDialogRequest) -> FileDialogAnswer) {
        let Some(p) = self.file_dialog.as_mut() else { return };
        if let Some(request) = p.request.take() {
            p.answer = Some(show(request));
        }
        let Some(answer) = p.answer.as_mut() else { return };
        let waker = Waker::from(Arc::new(Repaint(ctx.clone())));
        let Poll::Ready(path) = answer.as_mut().poll(&mut TaskContext::from_waker(&waker)) else { return };
        let Some(Pending { tab, then, .. }) = self.file_dialog.take() else { return };
        let Some(at) = self.docs.index_of(tab) else { return };
        let before = self.docs.active_id();
        if tab != before {
            crate::documents::switch(self, at);
        }
        let loading = self.file_load.busy();
        then(self, path);
        // A load started by `then` belongs to this tab: leaving it would discard the result.
        if tab != before
            && !(self.file_load.busy() && !loading)
            && self.docs.active_id() == tab
            && let Some(back) = self.docs.index_of(before)
        {
            crate::documents::switch(self, back);
        }
        ctx.request_repaint();
    }
}

#[cfg(test)]
pub(crate) mod test_support {
    use std::cell::RefCell;
    use std::rc::Rc;

    use super::FileDialogAnswer;

    /// An answer the test releases by hand (`None` inside: cancelled).
    pub(crate) type Slot = Rc<RefCell<Option<Option<String>>>>;

    /// A dialog that stays open until the test puts an answer in `slot`.
    pub(crate) fn manual(slot: &Slot) -> FileDialogAnswer {
        let slot = slot.clone();
        Box::pin(std::future::poll_fn(move |_| match slot.borrow_mut().take() {
            Some(answer) => std::task::Poll::Ready(answer),
            None => std::task::Poll::Pending,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::test_support::{Slot, manual};
    use super::*;
    use crate::{Services, SolveApp};
    use solidcraft_engine::Session;
    use std::cell::{Cell, RefCell};
    use std::rc::Rc;

    fn app() -> SolveApp {
        SolveApp::new(Session::default(), Services::default())
    }

    fn picked(app: &mut SolveApp) -> Rc<RefCell<Vec<String>>> {
        let got = Rc::new(RefCell::new(Vec::new()));
        let sink = got.clone();
        app.pick_open(move |_, p| sink.borrow_mut().push(p));
        got
    }

    #[test]
    fn a_picked_path_reaches_then_on_a_later_poll() {
        let ctx = egui::Context::default();
        let mut a = app();
        let slot: Slot = Rc::default();
        let shown = Cell::new(0);
        let got = picked(&mut a);
        assert!(a.file_dialog_open());
        assert!(got.borrow().is_empty(), "nothing runs during the request");

        a.poll_file_dialog(&ctx, |_| {
            shown.set(shown.get() + 1);
            manual(&slot)
        });
        assert!(got.borrow().is_empty() && a.file_dialog_open(), "pending: nothing runs");
        a.poll_file_dialog(&ctx, |_| {
            shown.set(shown.get() + 1);
            manual(&slot)
        });
        assert_eq!(shown.get(), 1, "shown once");

        *slot.borrow_mut() = Some(Some("/tmp/a.step".into()));
        a.poll_file_dialog(&ctx, |_| manual(&slot));
        assert_eq!(*got.borrow(), vec!["/tmp/a.step".to_string()]);
        assert!(!a.file_dialog_open());
    }

    #[test]
    fn the_request_reaches_the_host() {
        let ctx = egui::Context::default();
        let mut a = app();
        a.pick_save("part.step", &["step", "stp"], |_, _| {});
        let mut asked = None;
        a.poll_file_dialog(&ctx, |r| {
            asked = Some(r);
            Box::pin(std::future::pending())
        });
        assert!(matches!(asked, Some(FileDialogRequest::Save { name, extensions }) if name == "part.step" && extensions == ["step", "stp"]));
    }

    #[test]
    fn cancel_skips_then_but_runs_what_was_queued_after_it() {
        let ctx = egui::Context::default();
        let mut a = app();
        let got = picked(&mut a);
        let after = Rc::new(Cell::new(false));
        let flag = after.clone();
        assert!(a.after_file_dialog(move |_| flag.set(true)));
        a.poll_file_dialog(&ctx, |_| Box::pin(std::future::ready(None)));
        assert!(got.borrow().is_empty(), "Cancel does not run then");
        assert!(after.get(), "but the follow-up runs");
        assert!(!a.file_dialog_open());
        assert!(!a.after_file_dialog(|_| {}), "no dialog pending");
    }

    #[test]
    fn a_path_runs_then_before_the_follow_up() {
        let ctx = egui::Context::default();
        let mut a = app();
        let order = Rc::new(RefCell::new(Vec::new()));
        let first = order.clone();
        a.pick_open(move |_, _| first.borrow_mut().push("then"));
        let second = order.clone();
        assert!(a.after_file_dialog(move |_| second.borrow_mut().push("after")));
        a.poll_file_dialog(&ctx, |_| Box::pin(std::future::ready(Some("/x".into()))));
        assert_eq!(*order.borrow(), vec!["then", "after"]);
    }

    #[test]
    fn a_second_request_while_one_is_shown_is_refused() {
        let ctx = egui::Context::default();
        let mut a = app();
        let slot: Slot = Rc::default();
        let first = picked(&mut a);
        a.poll_file_dialog(&ctx, |_| manual(&slot));
        assert!(a.status.is_none());
        let second = picked(&mut a);
        assert!(matches!(&a.status, Some((m, _, true)) if m == ALREADY_OPEN));
        *slot.borrow_mut() = Some(Some("/first".into()));
        a.poll_file_dialog(&ctx, |_| manual(&slot));
        assert_eq!(*first.borrow(), vec!["/first".to_string()], "the first still completes");
        assert!(second.borrow().is_empty());
    }

    #[test]
    fn a_request_not_yet_shown_is_replaced() {
        let ctx = egui::Context::default();
        let mut a = app();
        let first = picked(&mut a);
        let second = picked(&mut a);
        assert!(a.status.is_none(), "no error: nothing was shown");
        a.poll_file_dialog(&ctx, |_| Box::pin(std::future::ready(Some("/new".into()))));
        assert!(first.borrow().is_empty());
        assert_eq!(*second.borrow(), vec!["/new".to_string()]);
    }

    #[test]
    fn an_answer_asks_for_another_frame() {
        let ctx = egui::Context::default();
        let mut a = app();
        let slot: Slot = Rc::default();
        picked(&mut a);
        for _ in 0..4 {
            ctx.run_ui(egui::RawInput::default(), |_| {}).textures_delta.clear();
        }
        a.poll_file_dialog(&ctx, |_| manual(&slot));
        assert!(!ctx.has_requested_repaint(), "nothing answered yet");
        *slot.borrow_mut() = Some(Some("/x".into()));
        a.poll_file_dialog(&ctx, |_| manual(&slot));
        assert!(ctx.has_requested_repaint(), "what the answer did gets drawn");
    }

    #[test]
    fn an_open_answer_for_a_background_design_still_loads() {
        let ctx = egui::Context::default();
        let mut a = app();
        // Frames are laid out, never painted.
        a.viewport.no_pixels = true;
        let file = std::env::temp_dir().join(format!("solidcraft-ask-open-bg-{}.solidcraft", std::process::id()));
        let _ = std::fs::remove_file(&file);
        let mut maker = app();
        for _ in 0..2 {
            maker.run("solid.box", serde_json::json!({"length": 10, "width": 10, "height": 10})).unwrap();
        }
        maker.run("file.save_as", serde_json::json!({"path": file.to_string_lossy()})).unwrap();
        a.run("solid.box", serde_json::json!({"length": 10, "width": 10, "height": 10})).unwrap();
        crate::documents::new_design(&mut a);
        crate::documents::switch(&mut a, 0);
        a.pick_open(|app, p| app.open_path_async(&p));
        let slot: Slot = Rc::default();
        a.poll_file_dialog(&ctx, |_| manual(&slot));
        crate::documents::new_design(&mut a);
        *slot.borrow_mut() = Some(Some(file.to_string_lossy().to_string()));
        a.poll_file_dialog(&ctx, |_| manual(&slot));
        assert!(a.file_load.busy(), "the load started");
        for _ in 0..200 {
            ctx.run_ui(egui::RawInput::default(), |ui| {
                a.logic(ui.ctx());
                a.ui(ui);
            })
            .textures_delta
            .clear();
            if !a.file_load.busy() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        assert!(!matches!(&a.status, Some((m, _, true)) if m.contains("discarded")), "not discarded: {:?}", a.status);
        assert!(a.file_load.idle());
        let loaded = (0..a.docs.count()).any(|i| {
            crate::documents::switch(&mut a, i);
            a.session.doc.features.len() == 2
        });
        assert!(loaded, "the file opened");
        let _ = std::fs::remove_file(&file);
    }

    fn ask_save_as(a: &mut SolveApp, file: &std::path::Path) -> Rc<Cell<bool>> {
        let _ = std::fs::remove_file(file);
        let ran = Rc::new(Cell::new(false));
        let flag = ran.clone();
        a.pick_save("x.solidcraft", &["solidcraft"], move |app, p| {
            flag.set(true);
            let _ = app.run("file.save_as", serde_json::json!({"path": p}));
        });
        ran
    }

    #[test]
    fn an_answer_applies_to_the_design_that_asked() {
        let ctx = egui::Context::default();
        let mut a = app();
        let file = std::env::temp_dir().join(format!("solidcraft-ask-tab-{}.solidcraft", std::process::id()));
        a.run("solid.box", serde_json::json!({"length": 10, "width": 10, "height": 10})).unwrap();
        ask_save_as(&mut a, &file);
        let slot: Slot = Rc::default();
        a.poll_file_dialog(&ctx, |_| manual(&slot));
        crate::documents::new_design(&mut a);
        assert_eq!(a.docs.active, 1);
        *slot.borrow_mut() = Some(Some(file.to_string_lossy().to_string()));
        a.poll_file_dialog(&ctx, |_| manual(&slot));
        assert_eq!(a.docs.active, 1, "the new tab is active again");
        assert!(a.session.doc.features.is_empty(), "the new design is untouched");
        let saved = std::fs::read(&file).ok().and_then(|b| solidcraft_engine::io::read_design(&b).ok());
        assert_eq!(saved.map(|d| d.features.len()), Some(1), "the asking design was saved");
        crate::documents::switch(&mut a, 0);
        assert!(!a.session.is_dirty(), "and it is the one marked saved");
        let _ = std::fs::remove_file(&file);
    }

    #[test]
    fn an_answer_for_a_closed_design_runs_nothing() {
        let ctx = egui::Context::default();
        let mut a = app();
        let file = std::env::temp_dir().join(format!("solidcraft-ask-closed-{}.solidcraft", std::process::id()));
        a.run("solid.box", serde_json::json!({"length": 10, "width": 10, "height": 10})).unwrap();
        crate::documents::new_design(&mut a);
        crate::documents::switch(&mut a, 0);
        let ran = ask_save_as(&mut a, &file);
        let after = Rc::new(Cell::new(false));
        let flag = after.clone();
        assert!(a.after_file_dialog(move |_| flag.set(true)));
        let slot: Slot = Rc::default();
        a.poll_file_dialog(&ctx, |_| manual(&slot));
        crate::documents::close(&mut a, 0, true);
        *slot.borrow_mut() = Some(Some(file.to_string_lossy().to_string()));
        a.poll_file_dialog(&ctx, |_| manual(&slot));
        assert!(!ran.get() && !after.get() && !file.exists(), "nothing runs");
        assert!(!a.file_dialog_open());
    }

    #[test]
    fn the_canvas_tool_starts_once_the_image_is_picked_and_asks_again_next_time() {
        let mut h = crate::scenario::Harness::new();
        h.step(&serde_json::json!({"make_image": "pic.png"})).unwrap();
        for _ in 0..2 {
            h.step(&serde_json::json!({"choose_file": "pic.png"})).unwrap();
            h.app.start("canvas.insert");
            assert!(h.app.tool.is_none() && h.app.file_dialog_open(), "asks for the image first");
            h.frames(3);
            assert_eq!(h.app.tool.as_ref().map(|t| t.cmd), Some("canvas.insert"), "starts once picked");
            assert!(!h.app.file_dialog_open());
            h.app.tool = None;
        }
        h.app.start("canvas.insert");
        h.frames(3);
        assert!(h.app.tool.is_none(), "Cancel starts nothing");
    }
}
