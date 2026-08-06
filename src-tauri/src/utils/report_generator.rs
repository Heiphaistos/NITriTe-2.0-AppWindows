use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct ReportSection {
    pub title: String,
    pub rows: Vec<Vec<String>>, // tableau de paires [label, value]
}

#[derive(Debug, Deserialize)]
pub struct ReportData {
    pub title: String,
    pub generated_at: String,
    pub sections: Vec<ReportSection>,
}

/// Génère un rapport HTML à partir de données structurées
#[tauri::command]
pub fn generate_html_report(data: ReportData) -> String {
    let mut html = format!(
        r#"<!DOCTYPE html>
<html lang="fr">
<head>
<meta charset="UTF-8">
<title>{title}</title>
<style>
* {{ box-sizing: border-box; margin: 0; padding: 0; }}
body {{ font-family: 'Segoe UI', sans-serif; background: #0f0f11; color: #e4e4e7; padding: 32px; }}
h1 {{ font-size: 26px; font-weight: 800; color: #f97316; margin-bottom: 4px; }}
.meta {{ font-size: 12px; color: #71717a; margin-bottom: 32px; font-family: monospace; }}
.section {{ background: #18181b; border: 1px solid #27272a; border-radius: 12px; padding: 20px; margin-bottom: 20px; }}
.section h2 {{ font-size: 15px; font-weight: 700; color: #f4f4f5; margin-bottom: 14px; border-bottom: 1px solid #27272a; padding-bottom: 8px; }}
table {{ width: 100%; border-collapse: collapse; }}
tr:nth-child(even) td {{ background: #1c1c1f; }}
td {{ padding: 7px 10px; font-size: 13px; border-bottom: 1px solid #27272a; }}
td:first-child {{ color: #a1a1aa; width: 40%; font-weight: 500; }}
td:last-child {{ color: #e4e4e7; font-family: monospace; font-size: 12px; }}
.footer {{ text-align: center; font-size: 11px; color: #52525b; margin-top: 32px; }}
</style>
</head>
<body>
<h1>{title}</h1>
<div class="meta">Généré le {generated_at} — NiTriTe</div>
"#,
        title = html_escape(&data.title),
        generated_at = html_escape(&data.generated_at),
    );

    for section in &data.sections {
        html.push_str(&format!(
            "<div class=\"section\"><h2>{}</h2><table>",
            html_escape(&section.title)
        ));
        for row in &section.rows {
            let label = row.first().map(|s| s.as_str()).unwrap_or("");
            let value = row.get(1).map(|s| s.as_str()).unwrap_or("");
            html.push_str(&format!(
                "<tr><td>{}</td><td>{}</td></tr>",
                html_escape(label),
                html_escape(value)
            ));
        }
        html.push_str("</table></div>");
    }

    html.push_str("<div class=\"footer\">NiTriTe — Rapport Diagnostic</div></body></html>");
    html
}

/// Génère un rapport Markdown
#[tauri::command]
pub fn generate_md_report(data: ReportData) -> String {
    let mut md = format!(
        "# {}\n\n_Généré le {} — NiTriTe_\n\n---\n\n",
        md_escape(&data.title), md_escape(&data.generated_at)
    );

    for section in &data.sections {
        md.push_str(&format!("## {}\n\n", md_escape(&section.title)));
        md.push_str("| Propriété | Valeur |\n|---|---|\n");
        for row in &section.rows {
            let label = row.first().map(|s| s.as_str()).unwrap_or("");
            let value = row.get(1).map(|s| s.as_str()).unwrap_or("");
            md.push_str(&format!("| {} | {} |\n", md_escape(label), md_escape(value)));
        }
        md.push('\n');
    }

    md
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
     .replace('<', "&lt;")
     .replace('>', "&gt;")
     .replace('"', "&quot;")
}

/// generate_html_report échappe déjà `title`/`label`/`value` (html_escape, testé
/// ci-dessous) mais sa fonction sœur generate_md_report n'échappait rien du tout —
/// un `|` non échappé dans une valeur de diagnostic (chemin, commande, description
/// de service...) casse la structure du tableau Markdown (colonne supplémentaire
/// injectée), et un HTML brut non échappé survit tel quel dans la plupart des
/// convertisseurs Markdown→HTML (XSS si ce .md est un jour rendu). `\` doit être
/// échappé en premier pour ne pas doubler l'échappement du `\|` qu'on insère ensuite.
fn md_escape(s: &str) -> String {
    s.replace('\\', "\\\\")
     .replace('|', "\\|")
     .replace('<', "&lt;")
     .replace('>', "&gt;")
     .replace('\r', "")
     .replace('\n', " ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn html_escape_ampersand() {
        assert_eq!(html_escape("A&B"), "A&amp;B");
    }

    #[test]
    fn html_escape_angle_brackets() {
        assert_eq!(html_escape("<script>"), "&lt;script&gt;");
        assert_eq!(html_escape("<img src=x onerror=alert(1)>"), "&lt;img src=x onerror=alert(1)&gt;");
    }

    #[test]
    fn html_escape_double_quote() {
        assert_eq!(html_escape(r#"say "hello""#), "say &quot;hello&quot;");
    }

    #[test]
    fn html_escape_combined_xss_payload() {
        let payload = r#"<script>alert("xss&injection")</script>"#;
        let escaped = html_escape(payload);
        // Raw dangerous chars must be gone
        assert!(!escaped.contains('<'));
        assert!(!escaped.contains('>'));
        assert!(!escaped.contains('"'));
        // '&' remains in entities (&amp; &lt; etc.) — verify entities are present
        assert!(escaped.contains("&lt;script&gt;"));
        assert!(escaped.contains("&amp;"));
        assert!(escaped.contains("&quot;"));
    }

    #[test]
    fn html_escape_clean_string_unchanged() {
        assert_eq!(html_escape("Hello World 123"), "Hello World 123");
        assert_eq!(html_escape(""), "");
    }

    #[test]
    fn generate_html_report_escapes_title() {
        let data = ReportData {
            title: "<XSS> & Test".to_string(),
            generated_at: "2026-01-01".to_string(),
            sections: vec![],
        };
        let html = generate_html_report(data);
        assert!(!html.contains("<XSS>"));
        assert!(html.contains("&lt;XSS&gt;"));
        assert!(html.contains("&amp;"));
    }

    #[test]
    fn generate_html_report_section_row_rendered() {
        let data = ReportData {
            title: "Rapport".to_string(),
            generated_at: "2026-01-01".to_string(),
            sections: vec![ReportSection {
                title: "Système".to_string(),
                rows: vec![
                    vec!["OS".to_string(), "Windows 11".to_string()],
                    vec!["RAM".to_string(), "16 Go".to_string()],
                ],
            }],
        };
        let html = generate_html_report(data);
        assert!(html.contains("Système"));
        assert!(html.contains("<td>OS</td>"));
        assert!(html.contains("Windows 11"));
        assert!(html.contains("16 Go"));
    }

    #[test]
    fn generate_html_report_row_value_xss_escaped() {
        let data = ReportData {
            title: "Test".to_string(),
            generated_at: "now".to_string(),
            sections: vec![ReportSection {
                title: "Section".to_string(),
                rows: vec![vec!["Key".to_string(), "<evil>".to_string()]],
            }],
        };
        let html = generate_html_report(data);
        assert!(!html.contains("<evil>"));
        assert!(html.contains("&lt;evil&gt;"));
    }

    #[test]
    fn generate_md_report_structure() {
        let data = ReportData {
            title: "Diagnostic".to_string(),
            generated_at: "2026-01-01".to_string(),
            sections: vec![ReportSection {
                title: "Réseau".to_string(),
                rows: vec![vec!["IP".to_string(), "192.168.1.1".to_string()]],
            }],
        };
        let md = generate_md_report(data);
        assert!(md.starts_with("# Diagnostic"));
        assert!(md.contains("## Réseau"));
        assert!(md.contains("| IP | 192.168.1.1 |"));
    }

    #[test]
    fn md_escape_pipe() {
        assert_eq!(md_escape("a|b"), "a\\|b");
    }

    #[test]
    fn md_escape_backslash_before_pipe_no_double_escape() {
        // Le \ doit être échappé en premier, sinon le \| inséré pour le pipe
        // serait lui-même ré-échappé en \\| par un remplacement ultérieur du \.
        assert_eq!(md_escape("a\\|b"), "a\\\\\\|b");
    }

    #[test]
    fn md_escape_newline_becomes_space() {
        assert_eq!(md_escape("line1\nline2"), "line1 line2");
    }

    #[test]
    fn md_escape_angle_brackets() {
        assert_eq!(md_escape("<script>"), "&lt;script&gt;");
    }

    #[test]
    fn generate_md_report_pipe_in_value_does_not_break_table_structure() {
        let data = ReportData {
            title: "Test".to_string(),
            generated_at: "now".to_string(),
            sections: vec![ReportSection {
                title: "Section".to_string(),
                rows: vec![vec!["Cmd".to_string(), "cmd.exe /c dir | more".to_string()]],
            }],
        };
        let md = generate_md_report(data);
        // Repro cycle 158 : un pipe non échappé produisait "| Cmd | cmd.exe /c dir | more |"
        // (4 segments -> colonne fantôme), au lieu d'une seule ligne à 2 colonnes.
        assert!(md.contains("| Cmd | cmd.exe /c dir \\| more |"));
        assert!(!md.contains("dir | more"));
    }

    #[test]
    fn generate_md_report_row_value_html_escaped() {
        let data = ReportData {
            title: "Test".to_string(),
            generated_at: "now".to_string(),
            sections: vec![ReportSection {
                title: "Section".to_string(),
                rows: vec![vec!["Key".to_string(), "<img src=x onerror=alert(1)>".to_string()]],
            }],
        };
        let md = generate_md_report(data);
        assert!(!md.contains("<img"));
        assert!(md.contains("&lt;img"));
    }

    #[test]
    fn generate_md_report_title_with_newline_does_not_inject_extra_heading() {
        let data = ReportData {
            title: "Test\n# Fake Heading".to_string(),
            generated_at: "now".to_string(),
            sections: vec![],
        };
        let md = generate_md_report(data);
        assert!(!md.contains("\n# Fake Heading"));
    }
}
