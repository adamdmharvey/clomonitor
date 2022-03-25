use super::{check, check::path::Globs};
use anyhow::Error;
use metadata::{Exemption, Metadata};
use octocrab::models::Repository;
use patterns::*;
use regex::{Regex, RegexSet};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

pub(crate) mod content;
pub(crate) mod git;
pub(crate) mod github;
pub(crate) mod license;
pub(crate) mod metadata;
pub(crate) mod path;
pub(crate) mod patterns;

/// Checks identifiers.
pub const ADOPTERS_CHECK_ID: &str = "adopters";
pub const ARTIFACTHUB_BADGE_CHECK_ID: &str = "artifacthub_badge";
pub const CHANGELOG_CHECK_ID: &str = "changelog";
pub const CODE_OF_CONDUCT_CHECK_ID: &str = "code_of_conduct";
pub const COMMUNITY_MEETING_CHECK_ID: &str = "community_meeting";
pub const CONTRIBUTING_CHECK_ID: &str = "contributing";
pub const DCO_CHECK_ID: &str = "dco";
pub const GOVERNANCE_CHECK_ID: &str = "governance";
pub const LICENSE_APPROVED_CHECK_ID: &str = "license_approved";
pub const LICENSE_SCANNING_CHECK_ID: &str = "license_scanning";
pub const LICENSE_SPDX_CHECK_ID: &str = "license_spdx_id";
pub const MAINTAINERS_CHECK_ID: &str = "maintainers";
pub const OPENSSF_BADGE_CHECK_ID: &str = "openssf_badge";
pub const README_CHECK_ID: &str = "readme";
pub const RECENT_RELEASE_CHECK_ID: &str = "recent_release";
pub const ROADMAP_CHECK_ID: &str = "roadmap";
pub const SBOM_CHECK_ID: &str = "sbom";
pub const SECURITY_POLICY_CHECK_ID: &str = "security_policy";
pub const SLACK_PRESENCE_CHECK_ID: &str = "slack_presence";
pub const TRADEMARK_DISCLAIMER_CHECK_ID: &str = "trademark_disclaimer";
pub const WEBSITE_CHECK_ID: &str = "website";

/// Information used by checks to perform their operations.
#[derive(Debug)]
#[non_exhaustive]
pub struct CheckOptions {
    pub root: PathBuf,
    pub url: String,
    pub md: Option<Metadata>,
    pub gh_md: Repository,
}

/// Check result information.
#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct CheckResult<T = ()> {
    pub passed: bool,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<T>,

    pub exempt: bool,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub exemption_reason: Option<String>,
}

impl<T> Default for CheckResult<T> {
    fn default() -> Self {
        Self {
            passed: false,
            url: None,
            value: None,
            exempt: false,
            exemption_reason: None,
        }
    }
}

impl<T> From<bool> for CheckResult<T> {
    fn from(passed: bool) -> Self {
        Self {
            passed,
            ..Default::default()
        }
    }
}

impl<T> From<Option<T>> for CheckResult<T> {
    fn from(value: Option<T>) -> Self {
        Self {
            passed: value.is_some(),
            value,
            ..Default::default()
        }
    }
}

impl<T> From<Exemption> for CheckResult<T> {
    fn from(exemption: Exemption) -> Self {
        Self {
            exempt: true,
            exemption_reason: Some(exemption.reason),
            ..Default::default()
        }
    }
}

impl<T> CheckResult<T> {
    /// Create a new CheckResult instance from the url provided.
    pub(crate) fn from_url(url: Option<String>) -> Self {
        Self {
            passed: url.is_some(),
            url,
            ..Default::default()
        }
    }

    /// Create a new CheckResult instance from the Github url built using the
    /// path provided.
    pub(crate) fn from_path(path: Option<PathBuf>, gh_md: &Repository) -> Self {
        match path {
            Some(path) => {
                let url = github::build_url(
                    &path,
                    &gh_md.owner.as_ref().unwrap().login,
                    &gh_md.name,
                    gh_md.default_branch.as_ref().unwrap(),
                );
                CheckResult::from_url(Some(url))
            }
            None => false.into(),
        }
    }

    /// Indicates if the check passed or was exempt.
    pub fn passed_or_exempt(&self) -> bool {
        self.passed || self.exempt
    }
}

/// Adopters check.
pub(crate) fn adopters(opts: &CheckOptions) -> Result<CheckResult, Error> {
    // Check exemption
    if let Some(exemption) = find_exemption(ADOPTERS_CHECK_ID, &opts.md) {
        return Ok(exemption.into());
    }

    // File in repo or reference in README file
    find_file_or_reference(opts, ADOPTERS_FILE, &*ADOPTERS_IN_README)
}

/// Artifact Hub badge check.
pub(crate) fn artifacthub_badge(opts: &CheckOptions) -> Result<CheckResult, Error> {
    // Check exemption
    if let Some(exemption) = find_exemption(ARTIFACTHUB_BADGE_CHECK_ID, &opts.md) {
        return Ok(exemption.into());
    }

    // Reference in README file
    Ok(CheckResult::from_url(readme_capture(
        &opts.root,
        vec![&*ARTIFACTHUB_URL],
    )?))
}

/// Changelog check.
pub(crate) async fn changelog(opts: &CheckOptions) -> Result<CheckResult, Error> {
    // Check exemption
    if let Some(exemption) = find_exemption(CHANGELOG_CHECK_ID, &opts.md) {
        return Ok(exemption.into());
    }

    // File in repo or reference in README file
    let r = find_file_or_reference(opts, CHANGELOG_FILE, &*CHANGELOG_IN_README)?;
    if r.passed {
        return Ok(r);
    }

    // Reference in last release
    if check::github::last_release_body_matches(&opts.url, &*CHANGELOG_IN_GH_RELEASE).await? {
        return Ok(true.into());
    }

    Ok(false.into())
}

/// Code of conduct check.
pub(crate) async fn code_of_conduct(opts: &CheckOptions) -> Result<CheckResult, Error> {
    // Check exemption
    if let Some(exemption) = find_exemption(CODE_OF_CONDUCT_CHECK_ID, &opts.md) {
        return Ok(exemption.into());
    }

    // File in repo or reference in README file
    let r = find_file_or_reference(opts, CODE_OF_CONDUCT_FILE, &*CODE_OF_CONDUCT_IN_README)?;
    if r.passed {
        return Ok(r);
    }

    // File in .github repo
    let url = check::github::has_community_health_file("CODE_OF_CONDUCT.md", &opts.gh_md).await?;
    Ok(CheckResult::from_url(url))
}

/// Community meeting check.
pub(crate) fn community_meeting(opts: &CheckOptions) -> Result<CheckResult, Error> {
    // Check exemption
    if let Some(exemption) = find_exemption(COMMUNITY_MEETING_CHECK_ID, &opts.md) {
        return Ok(exemption.into());
    }

    // Reference in README file
    Ok(readme_matches(&opts.root, &*COMMUNITY_MEETING_TEXT)?.into())
}

/// Contributing check.
pub(crate) async fn contributing(opts: &CheckOptions) -> Result<CheckResult, Error> {
    // Check exemption
    if let Some(exemption) = find_exemption(CONTRIBUTING_CHECK_ID, &opts.md) {
        return Ok(exemption.into());
    }

    // File in repo or reference in README file
    let r = find_file_or_reference(opts, CONTRIBUTING_FILE, &*CONTRIBUTING_IN_README)?;
    if r.passed {
        return Ok(r);
    }

    // File in .github repo
    let url = check::github::has_community_health_file("CONTRIBUTING.md", &opts.gh_md).await?;
    Ok(CheckResult::from_url(url))
}

/// Developer Certificate of Origin check.
pub(crate) async fn dco(opts: &CheckOptions) -> Result<CheckResult, Error> {
    // Check exemption
    if let Some(exemption) = find_exemption(DCO_CHECK_ID, &opts.md) {
        return Ok(exemption.into());
    }

    // DCO signature in commits
    if let Ok(passed) = check::git::commits_have_dco_signature(&opts.root) {
        if passed {
            return Ok(true.into());
        }
    }

    // DCO app reference in last closed PR
    if check::github::last_pr_has_dco_check(&opts.url).await? {
        return Ok(true.into());
    }

    Ok(false.into())
}

/// Governance check.
pub(crate) fn governance(opts: &CheckOptions) -> Result<CheckResult, Error> {
    // Check exemption
    if let Some(exemption) = find_exemption(GOVERNANCE_CHECK_ID, &opts.md) {
        return Ok(exemption.into());
    }

    // File in repo or reference in README file
    find_file_or_reference(opts, GOVERNANCE_FILE, &*GOVERNANCE_IN_README)
}

/// License check.
pub(crate) fn license(opts: &CheckOptions) -> Result<CheckResult<String>, Error> {
    // Check exemption
    if let Some(exemption) = find_exemption(LICENSE_SPDX_CHECK_ID, &opts.md) {
        return Ok(exemption.into());
    }

    // File in repo
    if let Some(spdx_id) = check::license::detect(Globs {
        root: &opts.root,
        patterns: LICENSE_FILE,
        case_sensitive: true,
    })? {
        return Ok(Some(spdx_id).into());
    }

    // License detected by Github
    if let Some(license) = &opts.gh_md.license {
        if license.spdx_id != "NOASSERTION" {
            return Ok(Some(license.spdx_id.to_owned()).into());
        }
    }

    Ok(false.into())
}

/// Approved license check.
pub(crate) fn license_approved(
    spdx_id: &Option<String>,
    opts: &CheckOptions,
) -> Result<CheckResult<bool>, Error> {
    // Check exemption
    if let Some(exemption) = find_exemption(LICENSE_APPROVED_CHECK_ID, &opts.md) {
        return Ok(exemption.into());
    }

    // SPDX id in list of approved licenses
    let mut approved: Option<bool> = None;
    if let Some(spdx_id) = &spdx_id {
        approved = Some(check::license::is_approved(spdx_id))
    }

    Ok(CheckResult {
        passed: approved.unwrap_or(false),
        value: approved,
        ..Default::default()
    })
}

/// License scanning check.
pub(crate) fn license_scanning(opts: &CheckOptions) -> Result<CheckResult, Error> {
    // Check exemption
    if let Some(exemption) = find_exemption(LICENSE_SCANNING_CHECK_ID, &opts.md) {
        return Ok(exemption.into());
    }

    // Scanning url in metadata file
    if let Some(md) = &opts.md {
        if let Some(license_scanning) = &md.license_scanning {
            if let Some(url) = &license_scanning.url {
                return Ok(CheckResult::from_url(Some(url.to_owned())));
            }
        }
    }

    // Reference in README file
    if let Some(url) = check::content::find(
        Globs {
            root: &opts.root,
            patterns: README_FILE,
            case_sensitive: true,
        },
        vec![&*FOSSA_URL, &*SNYK_URL],
    )? {
        return Ok(CheckResult::from_url(Some(url)));
    };

    Ok(false.into())
}

/// Maintainers check.
pub(crate) fn maintainers(opts: &CheckOptions) -> Result<CheckResult, Error> {
    // Check exemption
    if let Some(exemption) = find_exemption(MAINTAINERS_CHECK_ID, &opts.md) {
        return Ok(exemption.into());
    }

    // File in repo or reference in README file
    find_file_or_reference(opts, MAINTAINERS_FILE, &*MAINTAINERS_IN_README)
}

/// OpenSSF badge check.
pub(crate) fn openssf_badge(opts: &CheckOptions) -> Result<CheckResult, Error> {
    // Check exemption
    if let Some(exemption) = find_exemption(OPENSSF_BADGE_CHECK_ID, &opts.md) {
        return Ok(exemption.into());
    }

    // Reference in README file
    Ok(CheckResult::from_url(readme_capture(
        &opts.root,
        vec![&*OPENSSF_URL],
    )?))
}

/// Recent release check.
pub(crate) async fn recent_release(opts: &CheckOptions) -> Result<CheckResult, Error> {
    // Check exemption
    if let Some(exemption) = find_exemption(RECENT_RELEASE_CHECK_ID, &opts.md) {
        return Ok(exemption.into());
    }

    Ok(CheckResult::from_url(
        check::github::has_recent_release(&opts.url).await?,
    ))
}

/// Roadmap check.
pub(crate) fn roadmap(opts: &CheckOptions) -> Result<CheckResult, Error> {
    // Check exemption
    if let Some(exemption) = find_exemption(ROADMAP_CHECK_ID, &opts.md) {
        return Ok(exemption.into());
    }

    // File in repo or reference in README
    find_file_or_reference(opts, ROADMAP_FILE, &*ROADMAP_IN_README)
}

/// Readme check.
pub(crate) fn readme(opts: &CheckOptions) -> Result<CheckResult, Error> {
    // Check exemption
    if let Some(exemption) = find_exemption(README_CHECK_ID, &opts.md) {
        return Ok(exemption.into());
    }

    // File in repo
    if let Some(path) = check::path::find(readme_globs(&opts.root))? {
        return Ok(CheckResult::from_path(Some(path), &opts.gh_md));
    }

    Ok(false.into())
}

/// Software bill of materials (SBOM).
pub(crate) async fn sbom(opts: &CheckOptions) -> Result<CheckResult, Error> {
    // Check exemption
    if let Some(exemption) = find_exemption(SBOM_CHECK_ID, &opts.md) {
        return Ok(exemption.into());
    }

    // Asset in last release
    if let Some(last_release) = github::last_release(&opts.url).await? {
        if last_release
            .assets
            .iter()
            .any(|asset| SBOM_IN_GH_RELEASE.is_match(&asset.name))
        {
            return Ok(true.into());
        }
    }

    // Reference in README file
    Ok(readme_matches(&opts.root, &*SBOM_IN_README)?.into())
}

/// Security policy check.
pub(crate) async fn security_policy(opts: &CheckOptions) -> Result<CheckResult, Error> {
    // Check exemption
    if let Some(exemption) = find_exemption(SECURITY_POLICY_CHECK_ID, &opts.md) {
        return Ok(exemption.into());
    }

    // File in repo or reference in README file
    let r = find_file_or_reference(opts, SECURITY_POLICY_FILE, &*SECURITY_POLICY_IN_README)?;
    if r.passed {
        return Ok(r);
    }

    // File in .github repo
    let url = check::github::has_community_health_file("SECURITY.md", &opts.gh_md).await?;
    Ok(CheckResult::from_url(url))
}

/// Slack presence check.
pub(crate) fn slack_presence(opts: &CheckOptions) -> Result<CheckResult, Error> {
    // Check exemption
    if let Some(exemption) = find_exemption(SLACK_PRESENCE_CHECK_ID, &opts.md) {
        return Ok(exemption.into());
    }

    // Reference in README file
    Ok(readme_matches(&opts.root, &*SLACK_IN_README)?.into())
}

/// Trademark disclaimer check.
pub(crate) async fn trademark_disclaimer(opts: &CheckOptions) -> Result<CheckResult, Error> {
    // Check exemption
    if let Some(exemption) = find_exemption(TRADEMARK_DISCLAIMER_CHECK_ID, &opts.md) {
        return Ok(exemption.into());
    }

    // Trademark disclaimer in website setup in Github
    if let Some(url) = &opts.gh_md.homepage {
        if !url.is_empty() {
            return Ok(check::content::remote_matches(url, &*TRADEMARK_DISCLAIMER)
                .await?
                .into());
        }
    }

    Ok(false.into())
}

/// Website check.
pub(crate) fn website(opts: &CheckOptions) -> Result<CheckResult, Error> {
    // Check exemption
    if let Some(exemption) = find_exemption(WEBSITE_CHECK_ID, &opts.md) {
        return Ok(exemption.into());
    }

    // Website in Github
    if let Some(url) = &opts.gh_md.homepage {
        if !url.is_empty() {
            return Ok(CheckResult::from_url(Some(url.to_string())));
        }
    }

    Ok(false.into())
}

/// Check if the repository is exempt from passing the provided check.
fn find_exemption(check_id: &str, md: &Option<Metadata>) -> Option<Exemption> {
    if let Some(md) = md {
        if let Some(exemptions) = &md.exemptions {
            if let Some(exemption) = exemptions.iter().find(|e| e.check == check_id) {
                if !exemption.reason.is_empty() && exemption.reason != "~" {
                    return Some(exemption.clone());
                }
            }
        }
    }
    None
}

/// Check if a file matching the patterns provided is found in the repo or if
/// any of the regular expressions provided matches the README file content.
fn find_file_or_reference<P>(
    opts: &CheckOptions,
    patterns: P,
    re: &RegexSet,
) -> Result<CheckResult, Error>
where
    P: IntoIterator,
    P::Item: AsRef<str>,
{
    // File in repo
    if let Some(path) = check::path::find(Globs {
        root: &opts.root,
        patterns,
        case_sensitive: false,
    })? {
        return Ok(CheckResult::from_path(Some(path), &opts.gh_md));
    }

    // Reference in README file
    if readme_matches(&opts.root, re)? {
        return Ok(true.into());
    }

    Ok(false.into())
}

/// Check if the README file content matches any of the regular expressions
/// provided.
fn readme_matches(root: &Path, re: &RegexSet) -> Result<bool, Error> {
    check::content::matches(readme_globs(root), re)
}

/// Check if the README file content matches any of the regular expressions
/// provided, returning the value from the first capture group.
fn readme_capture(root: &Path, regexps: Vec<&Regex>) -> Result<Option<String>, Error> {
    check::content::find(readme_globs(root), regexps)
}

// Returns a Globs instance used to locate the README file.
fn readme_globs(root: &Path) -> Globs<impl IntoIterator<Item = &str>> {
    Globs {
        root,
        patterns: README_FILE,
        case_sensitive: true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn check_result_from_passed() {
        assert_eq!(
            CheckResult::<()>::from(true),
            CheckResult {
                passed: true,
                ..Default::default()
            }
        );
    }

    #[test]
    fn check_result_from_value_some() {
        assert_eq!(
            CheckResult::from(Some("value".to_string())),
            CheckResult {
                passed: true,
                value: Some("value".to_string()),
                ..Default::default()
            }
        );
    }

    #[test]
    fn check_result_from_value_none() {
        assert_eq!(
            CheckResult::<()>::from(None),
            CheckResult {
                passed: false,
                ..Default::default()
            }
        );
    }

    #[test]
    fn check_result_from_url_some() {
        assert_eq!(
            CheckResult::<()>::from_url(Some("url".to_string())),
            CheckResult {
                passed: true,
                url: Some("url".to_string()),
                ..Default::default()
            }
        );
    }

    #[test]
    fn check_result_from_url_none() {
        assert_eq!(
            CheckResult::<()>::from_url(None),
            CheckResult {
                passed: false,
                ..Default::default()
            }
        );
    }
}
