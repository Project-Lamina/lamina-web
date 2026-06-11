use std::error;
use log::{debug, error};
use std::collections::HashMap;
use std::fs;

/// Simple template engine for HTML templates
pub struct TemplateEngine {
    templates: HashMap<String, String>,
}

impl TemplateEngine {
    /// Create a new template engine and load all templates
    pub fn new() -> Result<Self, Box<dyn error::Error + Send + Sync>> {
        let mut templates = HashMap::new();

        // Load all template files
        let template_files = [
            "base.html",
            "lamina_homepage.html",
            "docs_index.html",
            "docs_getting_started.html",
            "docs_ir.html",
            "docs_cli.html",
            "docs_rust.html",
            "docs_irbuilder.html",
            "docs_targets.html",
            "docs_c_bindings.html",
            "docs_c_bindings_install.html",
            "docs_c_bindings_usage.html",
            "404.html",
            "components/footer.html",
            "components/docs_header.html",
            "components/docs_sidebar.html",
        ];

        for template_name in template_files.iter() {
            let template_path = format!("templates/{template_name}");
            match fs::read_to_string(&template_path) {
                Ok(content) => {
                    let content =
                        crate::components::codeblock::highlight_html_code_blocks(&content);
                    templates.insert(template_name.to_string(), content);
                    debug!("Loaded template: {template_name}");
                }
                Err(e) => {
                    error!("Failed to load template {template_name}: {e}");
                    return Err(format!("Failed to load template {template_name}: {e}").into());
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

        let mut result = template.clone();

        // Replace variables in the format {{variable_name}}
        for (key, value) in variables {
            let placeholder = format!("{{{{{key}}}}}");
            result = result.replace(&placeholder, value);
        }

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
