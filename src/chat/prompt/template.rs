pub struct TemplateVariables {
    user: String,
    bot: String,
    time: String,
    time_since: String,
}

impl TemplateVariables {
    pub fn new(user: String, bot: String, time: String, time_since: String) -> Self {
        Self {
            user,
            bot,
            time,
            time_since,
        }
    }

    /// Helper to substitute template placeholders in a string.
    pub fn substitute_template(&self, s: &str) -> String {
        s.replace("{user}", &self.user)
            .replace("{bot}", &self.bot)
            .replace("{time}", &self.time)
            .replace("{time_since}", &self.time_since)
    }

    /// Helper to substitute template placeholders in a string.
    pub fn substitute_optional_template(&self, s: Option<&str>) -> Option<String> {
        s.map(|s| self.substitute_template(s))
    }

    /// Helper to substitute template placeholders for a vector of strings.
    pub fn substitute_templates(&self, vec: &[String]) -> Vec<String> {
        vec.into_iter()
            .map(|s| Self::substitute_template(&self, &s))
            .collect()
    }

    /// Helper to substitute template placeholders for a vector of strings.
    pub fn substitute_optional_templates(&self, vec: Option<&[String]>) -> Option<Vec<String>> {
        vec.map(|vec| self.substitute_templates(vec))
    }
}
