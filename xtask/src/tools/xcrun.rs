use crate::tools::Runner;

use anyhow::{anyhow, Result};
use log::info;

pub(crate) struct XcrunRunner {
    runner: Runner,
}

impl XcrunRunner {
    pub(crate) fn new() -> Self {
        let runner = Runner::new("xcrun");

        XcrunRunner { runner }
    }

    pub(crate) fn notarize(
        &mut self,
        dist_zip: &str,
        apple_username: &str,
        apple_team_id: &str,
        notarization_password: &str,
    ) -> Result<()> {
        info!("Beginning notarization process...");
        self.runner
            .exec(
                &[
                    "notarytool",
                    "submit",
                    dist_zip,
                    "--apple-id",
                    apple_username,
                    "--team-id",
                    apple_team_id,
                    "--wait",
                    "--timeout",
                    "20m",
                ],
                &["--password", notarization_password],
                None,
                false,
            )
            .map_err(|e| {
                anyhow!(
                    "{}",
                    e.to_string()
                        .replace(notarization_password, "xxxx-xxxx-xxxx-xxxx")
                )
            })?;
        info!("Notarization successful.");
        Ok(())
    }
}
