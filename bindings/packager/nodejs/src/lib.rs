use napi::{
    threadsafe_function::{ErrorStrategy, ThreadsafeFunction, ThreadsafeFunctionCallMode},
    Error, JsFunction, Result, Status,
};

#[napi_derive::napi]
pub fn cli(args: Vec<String>, bin_name: Option<String>, callback: JsFunction) -> Result<()> {
    let function: ThreadsafeFunction<bool, ErrorStrategy::CalleeHandled> = callback
        .create_threadsafe_function(0, |ctx| ctx.env.get_boolean(ctx.value).map(|v| vec![v]))?;

    // we need to run in a separate thread so Node.js (e.g. vue-cli-plugin-tauri) consumers
    // can do work while `tauri dev` is running.
    std::thread::spawn(move || match cargo_packager::cli::try_run(args, bin_name) {
        Ok(_) => function.call(Ok(true), ThreadsafeFunctionCallMode::Blocking),
        Err(e) => function.call(
            Err(Error::new(Status::GenericFailure, format!("{:#}", e))),
            ThreadsafeFunctionCallMode::Blocking,
        ),
    });

    Ok(())
}

#[napi_derive::napi]
pub fn log_error(error: String) {
    tracing::error!("{}", error);
}
