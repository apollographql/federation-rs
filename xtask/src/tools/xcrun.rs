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
        self.runner.set_bash_descriptor(format!("xcrun notarytool submit {dist_zip} --apple-id {apple_username} --apple-team-id {apple_team_id} --password xxxx-xxxx-xxxx-xxxx --wait --timeout 20m"));
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
                    "--password",
                    notarization_password,
                    "--wait",
                    "--timeout",
                    "20m",
                ],
                None,
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
