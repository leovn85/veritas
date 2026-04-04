#![allow(non_upper_case_globals)]
#![allow(non_snake_case)]
#![allow(non_camel_case_types)]
macro_rules! subscribe_function {
    (
        $detour:ident,
        $target:expr,
        $reroute:ident
    ) => {
        (|| -> anyhow::Result<()> {
            $crate::subscribers::SUBSCRIBERS_ENABLE_FN.push(Box::new(|| -> anyhow::Result<()> {
                $detour.enable()?;
                Ok(())
            }));
            $detour.initialize(std::mem::transmute($target), $reroute)?;
            Ok(())
        })()
    };
}

macro_rules! enable_subscribers {
    () => {
        (|| -> anyhow::Result<()> {
            #[allow(unused_unsafe)]
            for func in unsafe { &mut $crate::subscribers::SUBSCRIBERS_ENABLE_FN } {
                func()?
            }
            Ok(())
        })()
    };
}
pub(crate) use enable_subscribers;
pub static mut SUBSCRIBERS_ENABLE_FN: Vec<Box<dyn FnMut() -> anyhow::Result<()>>> = Vec::new();

// Fix later
macro_rules! safe_call {
    ($body:expr) => {{
        #[allow(unused_must_use)]
        let _ = match microseh::try_seh(|| {
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| -> Result<()> { $body }))
        }) {
            Ok(Ok(Ok(()))) => Ok(()),
            Ok(Ok(Err(err))) => {
                let backtrace = std::backtrace::Backtrace::capture();
                let message = format!(
                    "{} returned error: {:?}\nBacktrace:\n{}",
                    function_name!(),
                    err,
                    backtrace
                );

                log::error!("{}", message);
                Err(err.context("safe_call body returned an error"))
            }
            Ok(Err(panic)) => {
                let backtrace = std::backtrace::Backtrace::capture();
                let msg = panic
                    .downcast_ref::<&str>()
                    .map(|s| format!("Panic message: {}", s))
                    .or_else(|| {
                        panic
                            .downcast_ref::<String>()
                            .map(|s| format!("Panic message: {}", s))
                    })
                    .unwrap_or_else(|| format!("Unknown panic: {:#?}", panic));

                let message = format!(
                    "{} panicked {}\nBacktrace:\n{}",
                    function_name!(),
                    msg,
                    backtrace
                );

                log::error!("{}", message);
                Err(anyhow::anyhow!(message).context("Panic occurred"))
            }
            Err(seh) => {
                let backtrace = std::backtrace::Backtrace::capture();
                let message = format!(
                    "{} triggered SEH exception: {:?}\nBacktrace:\n{}",
                    function_name!(),
                    seh,
                    backtrace
                );

                log::error!("{}", message);
                Err(anyhow::anyhow!(message).context("SEH occurred"))
            }
        };
    }};
}

pub mod battle;
pub mod relics;
