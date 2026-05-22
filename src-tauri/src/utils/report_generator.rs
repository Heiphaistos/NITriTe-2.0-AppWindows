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
        data.title, data.generated_at
    );

    for section in &data.sections {
        md.push_str(&format!("## {}\n\n", section.title));
        md.push_str("| Propriété | Valeur |\n|---|---|\n");
        for row in &section.rows {
            let label = row.first().map(|s| s.as_str()).unwrap_or("");
            let value = row.get(1).map(|s| s.as_str()).unwrap_or("");
            md.push_str(&format!("| {} | {} |\n", label, value));
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
