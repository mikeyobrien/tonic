use crate::ParityReport;

pub(crate) fn render_markdown(report: &ParityReport) -> String {
    let mut markdown = String::new();

    markdown.push_str("# LLVM Catalog Parity Report\n\n");
    markdown.push_str(&format!("- Catalog: `{}`\n", report.catalog));
    markdown.push_str(&format!("- tonic binary: `{}`\n\n", report.tonic_bin));

    markdown.push_str("## Summary\n\n");
    markdown.push_str("| Metric | Value |\n");
    markdown.push_str("| --- | ---: |\n");
    markdown.push_str(&format!(
        "| Active fixtures | {} |\n",
        report.summary.active_total
    ));
    markdown.push_str(&format!(
        "| Compile matches | {} |\n",
        report.summary.compile_matches
    ));
    markdown.push_str(&format!(
        "| Compile mismatches | {} |\n",
        report.summary.compile_mismatches
    ));
    markdown.push_str(&format!(
        "| Runtime fixtures evaluated | {} |\n",
        report.summary.runtime_total
    ));
    markdown.push_str(&format!(
        "| Runtime matches | {} |\n",
        report.summary.runtime_matches
    ));
    markdown.push_str(&format!(
        "| Runtime mismatches | {} |\n",
        report.summary.runtime_mismatches
    ));
    markdown.push_str(&format!(
        "| Total mismatches | {} |\n\n",
        report.summary.total_mismatches
    ));

    markdown.push_str("## Top failure causes\n\n");
    if report.top_failure_causes.is_empty() {
        markdown.push_str("No mismatches.\n\n");
    } else {
        markdown.push_str("| Count | Cause | Fixtures |\n");
        markdown.push_str("| ---: | --- | --- |\n");
        for cause in &report.top_failure_causes {
            markdown.push_str(&format!(
                "| {} | {} | {} |\n",
                cause.count,
                cause.reason,
                cause.fixtures.join(", ")
            ));
        }
        markdown.push('\n');
    }

    markdown.push_str("## Fixture mismatches\n\n");
    markdown.push_str("| Fixture | Phase | Reason |\n");
    markdown.push_str("| --- | --- | --- |\n");
    let mut any = false;
    for fixture in &report.fixtures {
        for mismatch in &fixture.mismatches {
            any = true;
            markdown.push_str(&format!(
                "| {} | {} | {} |\n",
                fixture.path, mismatch.phase, mismatch.reason
            ));
        }
    }

    if !any {
        markdown.push_str("| (none) | - | - |\n");
    }

    markdown
}
