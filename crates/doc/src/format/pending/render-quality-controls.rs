//! Saves CPU worker selection, linear 2x supersampling and denoising controls in render setups.
//! Older designs retain native-size output and the existing denoising settings; omitted
//! CPU worker selection uses the bounded automatic pool, with one worker available for serial work.
//! The last release warns about this newer design format and preserves fields it cannot read.
