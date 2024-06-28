use simcore::async_mode_enabled;

mod simulation;

async_mode_enabled! {
    mod async_tests;
}
