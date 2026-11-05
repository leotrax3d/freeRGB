//! Read-only ADLX discovery: lists AMD fans and their tuning state.
//! Never takes control or writes (owner diagnostics, prompt 07 block C).
fn main() {
    match freergb_gpu_adlx::Adlx::load() {
        Ok(mut adlx) => {
            for fan in adlx.discover() {
                println!("{fan:?}");
            }
        }
        Err(error) => {
            eprintln!("ADLX unavailable: {error}");
            std::process::exit(1);
        }
    }
}
