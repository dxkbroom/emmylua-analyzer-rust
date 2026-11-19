use std::path::Path;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Name of the busted project config file used for auto-detection.
pub const BUSTED_CONFIG_FILE: &str = ".busted";

#[derive(Serialize, Deserialize, Debug, JsonSchema, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct EmmyrcTest {
    /// Test framework whose built-in definitions are loaded.
    ///
    /// - `busted`: busted globals (`describe`, `it`, ...) plus luassert (`assert`, `spy`, `mock`, ...).
    /// - `luassert`: only luassert (`require("luassert")`, the extended `assert`).
    /// - `none`: load nothing and turn off auto-detection.
    ///
    /// When unset, busted is enabled automatically if a `.busted` file exists in a workspace root.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub framework: Option<EmmyrcTestFramework>,

    /// Framework enabled by auto-detection (not read from config).
    #[serde(skip)]
    #[schemars(skip)]
    pub detected_framework: Option<EmmyrcTestFramework>,
}

#[derive(Serialize, Deserialize, Debug, JsonSchema, Clone, Copy, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum EmmyrcTestFramework {
    /// Do not load any test framework definitions and disable auto-detection.
    #[serde(rename = "none")]
    Disabled,
    /// busted, including luassert.
    Busted,
    /// luassert only.
    Luassert,
}

/// Built-in definition bundles under `resources/addons/`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TestFrameworkAddon {
    Busted,
    Luassert,
}

impl TestFrameworkAddon {
    /// Directory name under `resources/addons/`.
    pub fn dir_name(&self) -> &'static str {
        match self {
            TestFrameworkAddon::Busted => "busted",
            TestFrameworkAddon::Luassert => "luassert",
        }
    }
}

impl EmmyrcTestFramework {
    /// Addon bundles this framework loads.
    pub fn addons(&self) -> &'static [TestFrameworkAddon] {
        match self {
            EmmyrcTestFramework::Disabled => &[],
            EmmyrcTestFramework::Busted => {
                &[TestFrameworkAddon::Busted, TestFrameworkAddon::Luassert]
            }
            EmmyrcTestFramework::Luassert => &[TestFrameworkAddon::Luassert],
        }
    }
}

impl EmmyrcTest {
    /// Framework in effect: the configured one, else the auto-detected one.
    pub fn effective_framework(&self) -> Option<EmmyrcTestFramework> {
        self.framework
            .or(self.detected_framework)
            .filter(|framework| *framework != EmmyrcTestFramework::Disabled)
    }

    /// Whether busted was enabled by auto-detection rather than config.
    pub fn is_auto_detected(&self) -> bool {
        self.framework.is_none() && self.detected_framework.is_some()
    }

    /// Auto-detect the test framework from marker files in the given workspace roots.
    ///
    /// Only runs when `framework` is not configured. Currently a `.busted` file in any root
    /// enables busted.
    pub fn detect_framework<'a>(&mut self, roots: impl IntoIterator<Item = &'a Path>) {
        self.detected_framework = None;
        if self.framework.is_some() {
            return;
        }
        if roots
            .into_iter()
            .any(|root| root.join(BUSTED_CONFIG_FILE).is_file())
        {
            log::info!("found {BUSTED_CONFIG_FILE}, enabling built-in busted definitions");
            self.detected_framework = Some(EmmyrcTestFramework::Busted);
        }
    }
}
