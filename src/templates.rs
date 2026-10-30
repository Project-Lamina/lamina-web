use crate::components::codeblock::highlight_html_code_blocks;
use log::{debug, error};
use std::collections::HashMap;
use std::{error, fs};

/// Simple template engine for HTML templates
pub struct TemplateEngine {
    templates: HashMap<String, String>,
}

impl TemplateEngine {
    /// Create a new template engine and load every template from `template_dir`.
    ///
    /// Called once at startup so a missing or malformed template fails the boot with
    /// a clear message, rather than panicking inside the first request that needs it.
    pub fn new(template_dir: &str) -> Result<Self, Box<dyn error::Error + Send + Sync>> {
        let mut templates = HashMap::new();

        // Load all template files
        let template_files = [
            "base.html",
            "lamina_homepage.html",
            "roadmap.html",
            "roadmap_artery.html",
            "docs_index.html",
            "docs_getting_started.html",
            "docs_ir.html",
            "docs_ir_instructions.html",
            "docs_output.html",
            "docs_runtime.html",
            "docs_lumir.html",
            "docs_camina.html",
            "docs_cli.html",
            "docs_rust.html",
            "docs_irbuilder.html",
            "docs_targets.html",
            "docs_abi.html",
            "docs_ras.html",
            "docs_weld.html",
            "docs_c_bindings.html",
            "docs_c_bindings_install.html",
            "docs_c_bindings_usage.html",
            "404.html",
            "components/footer.html",
            "components/docs_header.html",
            "components/docs_sidebar.html",
        ];

        for template_name in template_files.iter() {
            let template_path = format!("{template_dir}/{template_name}");
            match fs::read_to_string(&template_path) {
                Ok(content) => {
                    let content = highlight_html_code_blocks(&content);
                    templates.insert(template_name.to_string(), content);
                    debug!("Loaded template: {template_name}");
                }
                Err(e) => {
                    error!("Failed to load template {template_path}: {e}");
                    let cwd = std::env::current_dir()
                        .map(|p| p.display().to_string())
                        .unwrap_or_else(|_| "<unknown>".to_string());
                    return Err(format!(
                        "Failed to load template {template_path}: {e} \
                         (working directory: {cwd}; set template_dir or TEMPLATE_DIR \
                         to point at the templates directory)"
                    )
                    .into());
                }
            }
        }

        Ok(Self { templates })
    }

    /// Render a template with variables
    pub fn render(
        &self,
        template_name: &str,
        variables: &HashMap<String, String>,
    ) -> Result<String, Box<dyn error::Error + Send + Sync>> {
        let template = self
            .templates
            .get(template_name)
            .ok_or_else(|| format!("Template '{template_name}' not found"))?;

        // Component versions reach every template from one place, so no page
        // carries a version literal and no handler has to remember to pass one.
        let mut scope = crate::versions::template_variables();
        for (key, value) in variables {
            scope.insert(key.clone(), value.clone());
        }

        let mut result = template.clone();

        // Replace variables in the format {{variable_name}}
        for (key, value) in &scope {
            let placeholder = format!("{{{{{key}}}}}");
            result = result.replace(&placeholder, value);
        }

        result = resolve_conditionals(&result, &scope);

        // Handle shared components after variable substitution.
        result = self.process_includes(&result, variables)?;

        Ok(result)
    }

    /// Process component includes in templates
    fn process_includes(
        &self,
        content: &str,
        variables: &HashMap<String, String>,
    ) -> Result<String, Box<dyn error::Error + Send + Sync>> {
        let mut result = content.to_string();

        // Replace {{footer}} with footer component
        if result.contains("{{footer}}") {
            let footer_content = self.render_component("footer", &HashMap::new())?;
            result = result.replace("{{footer}}", &footer_content);
        }

        if result.contains("{{docs_header}}") {
            let header_content = self.render_component("docs_header", variables)?;
            result = result.replace("{{docs_header}}", &header_content);
        }

        if result.contains("{{docs_sidebar}}") {
            let sidebar_content = self.render_component("docs_sidebar", variables)?;
            result = result.replace("{{docs_sidebar}}", &sidebar_content);
        }

        Ok(result)
    }

    /// Render a component template
    pub fn render_component(
        &self,
        component_name: &str,
        variables: &HashMap<String, String>,
    ) -> Result<String, Box<dyn error::Error + Send + Sync>> {
        let template_name = format!("components/{component_name}.html");
        self.render(&template_name, variables)
    }

    /// Render the base template with content, meta description, and optional additional CSS
    pub fn render_base_with_meta_and_css(
        &self,
        title: &str,
        content: &str,
        meta_description: &str,
        additional_css: Option<&str>,
    ) -> Result<String, Box<dyn error::Error + Send + Sync>> {
        let mut variables = HashMap::new();
        variables.insert("title".to_string(), title.to_string());
        variables.insert("content".to_string(), content.to_string());
        variables.insert("meta_description".to_string(), meta_description.to_string());

        // Add additional CSS if provided
        let css_link = additional_css
            .map(|css| format!("<link rel=\"stylesheet\" href=\"{css}\">"))
            .unwrap_or_default();
        variables.insert("additional_css".to_string(), css_link);

        self.render("base.html", &variables)
    }
}

/// Resolve `{{#if name}}…{{else}}…{{/if}}`, where a variable counts as set only
/// when it is present and not empty. Blocks do not nest.
///
/// This is what keeps an unavailable version off the page: the whole sentence,
/// filename, or table cell that would have carried a number drops out instead of
/// rendering a gap or a stale literal.
fn resolve_conditionals(content: &str, variables: &HashMap<String, String>) -> String {
    const OPEN: &str = "{{#if ";
    const ELSE: &str = "{{else}}";
    const CLOSE: &str = "{{/if}}";

    let mut out = String::with_capacity(content.len());
    let mut rest = content;

    while let Some(start) = rest.find(OPEN) {
        let after_open = &rest[start + OPEN.len()..];
        let Some(name_end) = after_open.find("}}") else {
            break;
        };
        let name = after_open[..name_end].trim();
        let body = &after_open[name_end + 2..];
        let Some(close) = body.find(CLOSE) else {
            break;
        };

        let block = &body[..close];
        let (when_set, when_unset) = match block.find(ELSE) {
            Some(split) => (&block[..split], &block[split + ELSE.len()..]),
            None => (block, ""),
        };

        out.push_str(&rest[..start]);
        let is_set = variables.get(name).is_some_and(|value| !value.is_empty());
        out.push_str(if is_set { when_set } else { when_unset });
        rest = &body[close + CLOSE.len()..];
    }

    out.push_str(rest);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scope(pairs: &[(&str, &str)]) -> HashMap<String, String> {
        pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect()
    }

    #[test]
    fn a_set_variable_keeps_its_block() {
        let vars = scope(&[("lamina_version", "0.1.0")]);
        assert_eq!(
            resolve_conditionals(
                "release{{#if lamina_version}} 0.1.0{{/if}}, see below",
                &vars
            ),
            "release 0.1.0, see below"
        );
    }

    #[test]
    fn an_empty_variable_drops_its_block() {
        let vars = scope(&[("ras_version", "")]);
        assert_eq!(
            resolve_conditionals("release{{#if ras_version}} 0.1.5{{/if}}, see below", &vars),
            "release, see below"
        );
    }

    #[test]
    fn a_missing_variable_takes_the_else_branch() {
        let vars = scope(&[]);
        assert_eq!(
            resolve_conditionals("{{#if weld_version}}v1{{else}}&mdash;{{/if}}", &vars),
            "&mdash;"
        );
    }

    #[test]
    fn several_blocks_resolve_independently() {
        let vars = scope(&[("a", "1"), ("b", "")]);
        assert_eq!(
            resolve_conditionals("{{#if a}}A{{/if}}-{{#if b}}B{{else}}no B{{/if}}-end", &vars),
            "A-no B-end"
        );
    }

    #[test]
    fn an_unterminated_block_is_left_alone() {
        let vars = scope(&[("a", "1")]);
        let input = "{{#if a}}A";
        assert_eq!(resolve_conditionals(input, &vars), input);
    }
}
