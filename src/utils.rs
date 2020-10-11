use std::env;

/// Are we running in in a CI environment?
pub fn is_ci() -> bool {
    env::var("CI").is_ok() || env::var("TF_BUILD").is_ok()
}

#[cfg(feature = "colors")]
pub use console::style;

#[cfg(not(feature = "colors"))]
mod fake_colors {
    pub struct FakeStyledObject<D>(D);

    macro_rules! style_attr {
        ($($name:ident)*) => {
            $(
                #[inline]
                pub fn $name(self) -> FakeStyledObject<D> { self }
            )*
        }
    }

    impl<D> FakeStyledObject<D> {
        style_attr!(red green yellow cyan bold dim underlined);
    }

    impl<D: std::fmt::Display> std::fmt::Display for FakeStyledObject<D> {
        fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
            std::fmt::Display::fmt(&self.0, f)
        }
    }

    pub fn style<D>(val: D) -> FakeStyledObject<D> {
        FakeStyledObject(val)
    }
}

#[cfg(not(feature = "colors"))]
pub use self::fake_colors::*;
