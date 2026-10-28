use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll, Wake, Waker};

use insta::{allow_duplicates, allow_duplicates_async, assert_debug_snapshot};

#[cfg(feature = "filters")]
#[test]
fn test_basic_filter() {
    use insta::{assert_snapshot, with_settings};
    with_settings!({filters => vec![
        (r"\b[[:xdigit:]]{8}\b", "[SHORT_HEX]")
    ]}, {
        assert_snapshot!("Hello DEADBEEF!", @"Hello [SHORT_HEX]!");
    })
}

#[cfg(feature = "filters")]
#[test]
fn test_strip_ansi_escape_codes_setting() {
    use insta::{assert_snapshot, with_settings};
    with_settings!({strip_ansi_escape_codes => true}, {
        assert_snapshot!("\x1b[31mhello\x1b[0m world", @"hello world");
    })
}

#[cfg(feature = "filters")]
#[test]
fn test_strip_ansi_escape_codes_with_filters() {
    use insta::{assert_snapshot, with_settings};
    // ANSI stripping should happen before user-defined filters
    with_settings!({
        strip_ansi_escape_codes => true,
        filters => vec![
            (r"\bhello\b", "[GREETING]")
        ]
    }, {
        assert_snapshot!("\x1b[32mhello\x1b[0m world", @"[GREETING] world");
    })
}

#[cfg(feature = "filters")]
#[test]
fn test_strip_ansi_escape_codes_programmatic() {
    use insta::assert_snapshot;
    let mut settings = insta::Settings::clone_current();
    settings.set_strip_ansi_escape_codes(true);
    settings.bind(|| {
        assert_snapshot!(
            "\x1b[1m\x1b[31mERROR\x1b[0m: something \x1b[32mfailed\x1b[0m",
            @"ERROR: something failed"
        );
    });
}

#[cfg(feature = "json")]
#[test]
fn test_basic_suffixes() {
    for value in [1, 2, 3] {
        insta::with_settings!({snapshot_suffix => value.to_string()}, {
            insta::assert_json_snapshot!(&value);
        });
    }
}

#[test]
fn test_basic_duplicates_passes() {
    allow_duplicates! {
        for x in (0..10).step_by(2) {
            let is_even = x % 2 == 0;
            assert_debug_snapshot!(is_even, @"true");
        }
    }
}

#[test]
#[should_panic = "snapshot assertion for 'basic_duplicates_assertion_failed' failed in line"]
fn test_basic_duplicates_assertion_failed() {
    allow_duplicates! {
        for x in (0..10).step_by(3) {
            let is_even = x % 2 == 0;
            assert_debug_snapshot!(is_even, @"true");
        }
    }
}

/// Polls a future to completion on the current thread.
fn block_on<F: Future>(future: F) -> F::Output {
    struct NoopWaker;

    impl Wake for NoopWaker {
        fn wake(self: Arc<Self>) {}
    }

    let waker = Waker::from(Arc::new(NoopWaker));
    let mut cx = Context::from_waker(&waker);
    let mut future = Box::pin(future);
    loop {
        if let Poll::Ready(rv) = future.as_mut().poll(&mut cx) {
            return rv;
        }
    }
}

/// Returns `Poll::Pending` once, so the caller gets polled again.
struct YieldNow(bool);

impl Future for YieldNow {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<()> {
        if self.0 {
            Poll::Ready(())
        } else {
            self.0 = true;
            Poll::Pending
        }
    }
}

#[test]
fn test_async_duplicates_passes() {
    block_on(allow_duplicates_async! {
        for x in (0..10).step_by(2) {
            YieldNow(false).await;
            let is_even = x % 2 == 0;
            assert_debug_snapshot!(is_even, @"true");
        }
    });
}

#[test]
#[should_panic = "Result does not match previous snapshot in allow-duplicates block"]
fn test_async_duplicates_assertion_failed() {
    block_on(allow_duplicates_async! {
        for x in (0..10).step_by(3) {
            YieldNow(false).await;
            let is_even = x % 2 == 0;
            assert_debug_snapshot!(is_even, @"true");
        }
    });
}
