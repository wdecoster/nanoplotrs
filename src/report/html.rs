//! HTML report generation using maud

use crate::config::Config;
use crate::error::Result;
use crate::plots::GeneratedPlot;
use crate::stats::Stats;
use maud::{html, Markup, DOCTYPE, PreEscaped};
use std::fs::File;
use std::io::Write;

/// Generate the HTML report with embedded plots
pub fn generate_html_report(
    plots: &[GeneratedPlot],
    stats: &Stats,
    stats_before_filter: Option<&Stats>,
    config: &Config,
) -> Result<()> {
    let html_content = build_html(plots, stats, stats_before_filter, config);

    let report_path = config.output_path("NanoPlot-report.html");
    let mut file = File::create(&report_path)?;
    file.write_all(html_content.into_string().as_bytes())?;

    Ok(())
}

fn build_html(
    plots: &[GeneratedPlot],
    stats: &Stats,
    stats_before_filter: Option<&Stats>,
    _config: &Config,
) -> Markup {
    html! {
        (DOCTYPE)
        html {
            head {
                meta charset="UTF-8";
                title { "NanoPlot Report" }
                style { (css_styles()) }
            }
            body {
                div class="grid" {
                    (build_header(plots, stats_before_filter.is_some()))
                    main class="grid-main" {
                        h2 { "NanoPlot Report" }

                        @if let Some(pre_stats) = stats_before_filter {
                            h3 id="stats-before" { "Summary statistics prior to filtering" }
                            (build_stats_table(pre_stats))

                            h3 id="stats-after" { "Summary statistics after filtering" }
                            (build_stats_table(stats))
                        } @else {
                            h3 id="stats" { "Summary statistics" }
                            (build_stats_table(stats))
                        }

                        h3 id="plots" { "Plots" }
                        @for plot in plots {
                            (build_plot_section(plot))
                        }
                    }
                }
                (build_javascript())
            }
        }
    }
}

fn build_header(plots: &[GeneratedPlot], has_filter: bool) -> Markup {
    html! {
        header class="grid-header" {
            nav {
                h2 class="hiddentitle" { "Menu" }
                ul {
                    @if has_filter {
                        li { a href="#stats-before" { "Statistics (before filter)" } }
                        li { a href="#stats-after" { "Statistics (after filter)" } }
                    } @else {
                        li { a href="#stats" { "Statistics" } }
                    }
                    li class="submenu" {
                        a href="#plots" class="submenubtn" { "Plots" }
                        ul class="submenu-items" {
                            @for plot in plots {
                                li {
                                    a href=(format!("#{}", plot.title.replace(' ', "_"))) {
                                        (plot.title.clone())
                                    }
                                }
                            }
                        }
                    }
                    li class="issue-btn" {
                        a href="https://github.com/wdecoster/nanoplotrs/issues" target="_blank" class="reporting" {
                            "Report issue"
                        }
                    }
                }
            }
        }
    }
}

fn build_stats_table(stats: &Stats) -> Markup {
    html! {
        table {
            tbody {
                tr {
                    td { "Number of reads" }
                    td { (format_number(stats.num_reads as u64)) }
                }
                tr {
                    td { "Total bases" }
                    td { (format_number(stats.total_bases)) }
                }
                tr {
                    td { "Median read length" }
                    td { (format!("{:.1}", stats.median_length)) }
                }
                tr {
                    td { "Mean read length" }
                    td { (format!("{:.1}", stats.mean_length)) }
                }
                tr {
                    td { "STDEV read length" }
                    td { (format!("{:.1}", stats.stdev_length)) }
                }
                tr {
                    td { "Min read length" }
                    td { (stats.min_length) }
                }
                tr {
                    td { "Max read length" }
                    td { (format_number(stats.max_length as u64)) }
                }
                tr {
                    td { "Read length N50" }
                    td { (format_number(stats.n50)) }
                }
                @if let Some(mean_q) = stats.mean_quality {
                    tr {
                        td { "Mean read quality" }
                        td { (format!("{:.1}", mean_q)) }
                    }
                }
                @if let Some(median_q) = stats.median_quality {
                    tr {
                        td { "Median read quality" }
                        td { (format!("{:.1}", median_q)) }
                    }
                }
                @if let Some(aligned) = stats.total_aligned_bases {
                    tr {
                        td { "Total aligned bases" }
                        td { (format_number(aligned)) }
                    }
                }
                @if let Some(mean_pi) = stats.mean_percent_identity {
                    tr {
                        td { "Mean percent identity" }
                        td { (format!("{:.2}%", mean_pi)) }
                    }
                }
                @if let Some(median_pi) = stats.median_percent_identity {
                    tr {
                        td { "Median percent identity" }
                        td { (format!("{:.2}%", median_pi)) }
                    }
                }
            }
        }
    }
}

fn build_plot_section(plot: &GeneratedPlot) -> Markup {
    let anchor = plot.title.replace(' ', "_");

    html! {
        button class="collapsible" { (&plot.title) }
        section class="collapsible-content" {
            h4 class="hiddentitle" id=(anchor) { (&plot.title) }
            div class="plot-container" {
                (PreEscaped(&plot.svg_content))
            }
        }
    }
}

fn build_javascript() -> Markup {
    html! {
        script {
            (PreEscaped(r#"
var coll = document.getElementsByClassName("collapsible");
for (var i = 0; i < coll.length; i++) {
    coll[i].addEventListener("click", function() {
        this.classList.toggle("active");
        var content = this.nextElementSibling;
        if (content.style.display === "none") {
            content.style.display = "block";
        } else {
            content.style.display = "none";
        }
    });
}
"#))
        }
    }
}

fn css_styles() -> &'static str {
    r#"
body { margin: 0; font-family: Arial, sans-serif; }

.grid {
    display: grid;
    grid-template-areas: 'gheader' 'gmain';
    margin: 0;
}

.grid > .grid-header { grid-area: gheader; }
.grid > .grid-main { grid-area: gmain; padding: 20px; }

nav { text-align: center; }

ul {
    border-bottom: 1px solid white;
    list-style-type: none;
    margin: 0;
    padding: 0;
    overflow: hidden;
    background-color: #001f3f;
    font-size: 1.2em;
}

ul > li > ul { font-size: 1em; }

li { float: left; }

li a, .submenubutton {
    display: inline-block;
    color: white;
    text-align: center;
    padding: 14px 16px;
    text-decoration: none;
}

li a:hover, .submenu:hover .submenubutton {
    background-color: #39CCCC;
}

.submenu { display: inline-block; }

.submenu-items {
    display: none;
    position: absolute;
    background-color: #f9f9f9;
    min-width: 160px;
    z-index: 1;
}

.submenu-items li {
    display: block;
    float: none;
    overflow: hidden;
}

.submenu-items li a {
    color: black;
    padding: 12px 16px;
    text-decoration: none;
    display: block;
    text-align: left;
}

.submenu-items a:hover { background-color: #f1f1f1; }

.submenu:hover .submenu-items {
    display: block;
    float: bottom;
    overflow: hidden;
}

li { border-right: 1px solid #bbb; }

.issue-btn {
    border-right: none;
    float: right;
}

.hiddentitle {
    position: absolute;
    width: 1px;
    height: 1px;
    overflow: hidden;
    left: -10000px;
}

h2 {
    color: #111;
    font-size: 2.5em;
    font-weight: bold;
    text-align: center;
}

h3 {
    color: #111;
    font-size: 1.5em;
    font-weight: 300;
    text-align: center;
    padding-bottom: 0;
}

table {
    border-collapse: collapse;
    width: 100%;
    max-width: 600px;
    margin: 20px auto;
}

table td {
    border: 1px solid #ddd;
    padding: 8px;
}

table tr:nth-child(even) { background-color: #f2f2f2; }
table tr:hover { background-color: #ddd; }

table td:first-child { font-weight: bold; }
table td:last-child { text-align: right; }

.collapsible {
    background-color: #39CCCC;
    color: white;
    cursor: pointer;
    padding: 18px;
    width: 100%;
    border: none;
    text-align: left;
    outline: none;
    font-size: 15px;
}

.active, .collapsible:hover {
    color: white;
    background-color: #001f3f;
}

.collapsible-content {
    padding: 0 18px;
    display: block;
    overflow: hidden;
    background-color: #FFFFFF;
    text-align: center;
}

.collapsible:after {
    content: '-';
    font-size: 20px;
    font-weight: bold;
    float: right;
    color: white;
    margin-left: 5px;
}

.active:after { content: '+'; color: white; }

.plot-container {
    max-width: 100%;
    overflow-x: auto;
    padding: 20px 0;
}

.plot-container svg {
    max-width: 100%;
    height: auto;
}
"#
}

/// Format large numbers with commas
fn format_number(n: u64) -> String {
    let s = n.to_string();
    let mut result = String::new();
    let chars: Vec<char> = s.chars().collect();

    for (i, c) in chars.iter().enumerate() {
        if i > 0 && (chars.len() - i) % 3 == 0 {
            result.push(',');
        }
        result.push(*c);
    }

    result
}
