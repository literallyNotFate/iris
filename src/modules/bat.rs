use crate::{core::IrisContext, models::Palette, modules::ConfigGenerator, utils::Status};
use anyhow::{Context, Result};
use std::{fs, path::PathBuf, process::Command};

/// Config generator for bat
pub struct BatGenerator;

impl ConfigGenerator for BatGenerator {
    fn name(&self) -> &str {
        "bat"
    }

    fn apply(&self, p: &Palette, ctx: &IrisContext) -> Result<()> {
        let iris_bat_dir = ctx.paths.cache.join("bat_themes");
        fs::create_dir_all(&iris_bat_dir)?;

        let config_task = Status::step("Locating bat configuration...", 2);
        let bat_config_dir = Command::new("bat")
            .arg("--config-dir")
            .output()
            .map(|o| PathBuf::from(String::from_utf8_lossy(&o.stdout).trim()))
            .context("Failed to get bat config dir")?;

        let bat_themes_dir = bat_config_dir.join("themes");
        let link_path = bat_themes_dir.join("iris_themes");

        if !bat_themes_dir.exists() {
            fs::create_dir_all(&bat_themes_dir)?;
        }
        config_task.done(Some("Bat configuration directory located!"));

        if !link_path.exists() {
            #[cfg(unix)]
            {
                let link_task = Status::step("Linking iris themes to bat...", 2);
                std::os::unix::fs::symlink(&iris_bat_dir, &link_path)?;
                link_task.done(Some("Symlink created."));
            }
        }

        let theme_name = &ctx.state.current_theme;
        let theme_dir = ctx.paths.cache.join("bat_themes");
        std::fs::create_dir_all(&theme_dir).context("Failed to create bat theme directory")?;

        let rules = self.build_config(p);

        let content = format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
    <!DOCTYPE plist PUBLIC "-//Apple Computer//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
    <plist version="1.0">
    <dict>
        <key>name</key><string>{name}</string>
        <key>settings</key>
        <array>
    {rules}
        </array>
    </dict>
    </plist>"#,
            name = theme_name,
            rules = rules
        );

        let theme_file = theme_dir.join(format!("{}.tmTheme", theme_name));
        std::fs::write(&theme_file, content).context("Failed to write theme file")?;

        let config_file = ctx.paths.cache.join("bat.conf");
        let bat_config = format!(
            "--theme=\"{name}\"\n--style=\"numbers,changes\"\n--color=\"always\"\n",
            name = theme_name
        );
        std::fs::write(config_file, bat_config).context("Failed to write bat.conf")?;

        let cache_task = Status::step("Rebuilding bat cache...", 2);
        let output = std::process::Command::new("bat")
            .arg("cache")
            .arg("--build")
            .output()?;

        if output.status.success() {
            cache_task.done(Some("Bat cache updated."));
        } else {
            let err = String::from_utf8_lossy(&output.stderr);
            cache_task.fail(&err);
            anyhow::bail!("Bat cache build failed");
        }

        Ok(())
    }
}

impl BatGenerator {
    /// Generate .tmTheme for bat
    pub fn build_config(&self, p: &Palette) -> String {
        let mut s = String::new();
        let c = |hex: &str| format!("#{}", hex.trim_start_matches('#'));
        let xml_safe = |t: &str| {
            t.replace("&", "&amp;")
                .replace("<", "&lt;")
                .replace(">", "&gt;")
        };

        s.push_str(&format!(
            r#"    <dict>
          <key>settings</key>
          <dict>
            <key>background</key><string>{bg}</string>
            <key>foreground</key><string>{fg}</string>
            <key>caret</key><string>{fg}</string>
            <key>lineHighlight</key><string>{line}</string>
            <key>selection</key><string>{sel}</string>
          </dict>
        </dict>
    "#,
            bg = c(&p.bg),
            fg = c(&p.fg),
            line = c(&p.line_hl),
            sel = c(&p.sel)
        ));

        let mut add = |name: &str, scope: &str, color: &str, style: &str| {
            let style_xml = if style.is_empty() {
                "<string/>".to_string()
            } else {
                format!("<string>{}</string>", style)
            };

            s.push_str(&format!(
                r#"    <dict>
          <key>name</key><string>{name}</string>
          <key>scope</key><string>{scope}</string>
          <key>settings</key>
          <dict>
            <key>foreground</key><string>{color}</string>
            <key>fontStyle</key>{style_xml}
          </dict>
        </dict>
    "#,
                name = xml_safe(name),
                scope = xml_safe(scope),
                color = c(color),
                style_xml = style_xml
            ));
        };

        add(
            "Basic text & variable names (incl. leading punctuation)",
            "text, source, variable.other.readwrite, punctuation.definition.variable",
            &p.fg,
            "",
        );
        add(
            "Parentheses, Brackets, Braces",
            "punctuation",
            &p.comment,
            "",
        );
        add(
            "Comments",
            "comment, punctuation.definition.comment",
            &p.comment,
            "italic",
        );
        add("", "string, punctuation.definition.string", &p.string, "");
        add("", "constant.character.escape", &p.fg, "");
        add(
            "Booleans, constants, numbers",
            "constant.numeric, variable.other.constant, entity.name.constant, constant.language.boolean, constant.language.false, constant.language.true, keyword.other.unit.user-defined, keyword.other.unit.suffix.floating-point",
            &p.number,
            "",
        );
        add(
            "",
            "keyword, keyword.operator.word, keyword.operator.new, variable.language.super, support.type.primitive, storage.type, storage.modifier, punctuation.definition.keyword",
            &p.keyword,
            "",
        );
        add("", "entity.name.tag.documentation", &p.keyword, "");
        add(
            "Punctuation",
            "keyword.operator, punctuation.accessor, punctuation.definition.generic, meta.function.closure punctuation.section.parameters, punctuation.definition.tag, punctuation.separator.key-value",
            &p.operator,
            "",
        );
        add(
            "",
            "entity.name.function, meta.function-call.method, support.function, support.function.misc, variable.function",
            &p.func,
            "italic",
        );
        add(
            "Classes",
            "entity.name.class, entity.other.inherited-class, support.class, meta.function-call.constructor, entity.name.struct",
            &p.type_name,
            "italic",
        );
        add("Enum", "entity.name.enum", &p.type_name, "italic");
        add(
            "Enum member",
            "meta.enum variable.other.readwrite, variable.other.enummember",
            &p.operator,
            "",
        );
        add("Object properties", "meta.property.object", &p.operator, "");
        add(
            "Types",
            "meta.type, meta.type-alias, support.type, entity.name.type",
            &p.type_name,
            "italic",
        );
        add(
            "Decorators",
            "meta.annotation variable.function, meta.annotation variable.annotation.function, meta.annotation punctuation.definition.annotation, meta.decorator, punctuation.decorator",
            &p.number,
            "",
        );
        add(
            "",
            "variable.parameter, meta.function.parameters",
            &p.attribute,
            "italic",
        );
        add(
            "Built-ins",
            "constant.language, support.function.builtin",
            &p.keyword,
            "",
        );
        add(
            "",
            "entity.other.attribute-name.documentation",
            &p.keyword,
            "",
        );
        add(
            "Preprocessor directives",
            "keyword.control.directive, punctuation.definition.directive",
            &p.type_name,
            "",
        );
        add(
            "Type parameters",
            "punctuation.definition.typeparameters",
            &p.func,
            "",
        );
        add("Namespaces", "entity.name.namespace", &p.type_name, "");
        add(
            "Property names (left hand assignments in json/yaml/css)",
            "support.type.property-name.css",
            &p.func,
            "",
        );
        add(
            "This/Self keyword",
            "variable.language.this, variable.language.this punctuation.definition.variable",
            &p.keyword,
            "",
        );
        add("Object properties", "variable.object.property", &p.fg, "");
        add(
            "String template interpolation",
            "string.template variable, string variable",
            &p.fg,
            "",
        );
        add(
            "C++ extern keyword",
            "storage.modifier.specifier.extern.cpp",
            &p.keyword,
            "",
        );
        add(
            "C++ scope resolution",
            "entity.name.scope-resolution.template.call.cpp, entity.name.scope-resolution.parameter.cpp, entity.name.scope-resolution.cpp, entity.name.scope-resolution.function.definition.cpp",
            &p.type_name,
            "",
        );
        add(
            "C++ operators",
            "storage.modifier.reference.cpp",
            &p.operator,
            "",
        );
        add(
            "C# Interpolated Strings",
            "meta.interpolation.cs",
            &p.fg,
            "",
        );
        add(
            "C# xml-style docs",
            "comment.block.documentation.cs",
            &p.fg,
            "",
        );
        add(
            "Classes, reflecting the className color in JSX",
            "source.css entity.other.attribute-name.class.css, entity.other.attribute-name.parent-selector.css punctuation.definition.entity.css",
            &p.type_name,
            "",
        );
        add(
            "Operators",
            "punctuation.separator.operator.css",
            &p.operator,
            "",
        );
        add(
            "Pseudo classes",
            "source.css entity.other.attribute-name.pseudo-class",
            &p.operator,
            "",
        );
        add("", "source.css constant.other.unicode-range", &p.number, "");
        add("", "source.css variable.parameter.url", &p.string, "");
        add(
            "CSS vendored property names",
            "support.type.vendored.property-name",
            &p.func,
            "",
        );
        add(
            "Less/SCSS right-hand variables (@/$-prefixed)",
            "source.css meta.property-value variable, source.css meta.property-value variable.other.less, source.css meta.property-value variable.other.less punctuation.definition.variable.less, meta.definition.variable.scss",
            &p.attribute,
            "",
        );
        add(
            "CSS variables (--prefixed)",
            "source.css meta.property-list variable, meta.property-list variable.other.less, meta.property-list variable.other.less punctuation.definition.variable.less",
            &p.func,
            "",
        );
        add(
            "CSS Percentage values, styled the same as numbers",
            "keyword.other.unit.percentage.css",
            &p.number,
            "",
        );
        add(
            "CSS Attribute selectors, styled the same as strings",
            "source.css meta.attribute-selector",
            &p.string,
            "",
        );
        add(
            "JSON/YAML keys, other left-hand assignments",
            "keyword.other.definition.ini, punctuation.support.type.property-name.json, support.type.property-name.json, punctuation.support.type.property-name.toml, support.type.property-name.toml, entity.name.tag.yaml, punctuation.support.type.property-name.yaml, support.type.property-name.yaml",
            &p.func,
            "",
        );
        add(
            "JSON/YAML constants",
            "constant.language.json, constant.language.yaml",
            &p.number,
            "",
        );
        add(
            "YAML anchors",
            "entity.name.type.anchor.yaml, variable.other.alias.yaml",
            &p.type_name,
            "",
        );
        add(
            "TOML tables / ini groups",
            "support.type.property-name.table, entity.name.section.group-title.ini",
            &p.type_name,
            "",
        );
        add(
            "TOML dates",
            "constant.other.time.datetime.offset.toml",
            &p.fg,
            "",
        );
        add(
            "YAML anchor puctuation",
            "punctuation.definition.anchor.yaml, punctuation.definition.alias.yaml",
            &p.fg,
            "",
        );
        add(
            "YAML triple dashes",
            "entity.other.document.begin.yaml",
            &p.fg,
            "",
        );
        add("Markup Diff", "markup.changed.diff", &p.number, "");
        add(
            "Diff",
            "meta.diff.header.from-file, meta.diff.header.to-file, punctuation.definition.from-file.diff, punctuation.definition.to-file.diff",
            &p.func,
            "",
        );
        add("Diff Inserted", "markup.inserted.diff", &p.string, "");
        add("Diff Deleted", "markup.deleted.diff", &p.keyword, "");
        add(
            "dotenv left-hand side assignments",
            "variable.other.env",
            &p.func,
            "",
        );
        add(
            "dotenv reference to existing env variable",
            "string.quoted variable.other.env",
            &p.fg,
            "",
        );
        add(
            "GDScript functions",
            "support.function.builtin.gdscript",
            &p.func,
            "",
        );
        add(
            "GDScript constants",
            "constant.language.gdscript",
            &p.number,
            "",
        );
        add(
            "Comment keywords",
            "comment meta.annotation.go",
            &p.attribute,
            "",
        );
        add(
            "go:embed, go:build, etc.",
            "comment meta.annotation.parameters.go",
            &p.number,
            "",
        );
        add(
            "Go constants (nil, true, false)",
            "constant.language.go",
            &p.number,
            "",
        );
        add("GraphQL variables", "variable.graphql", &p.fg, "");
        add(
            "GraphQL aliases",
            "string.unquoted.alias.graphql",
            &p.attribute,
            "",
        );
        add(
            "GraphQL enum members",
            "constant.character.enum.graphql",
            &p.operator,
            "",
        );
        add(
            "GraphQL field in types",
            "meta.objectvalues.graphql constant.object.key.graphql string.unquoted.graphql",
            &p.attribute,
            "",
        );
        add(
            "HTML/XML DOCTYPE as keyword",
            "keyword.other.doctype, meta.tag.sgml.doctype punctuation.definition.tag, meta.tag.metadata.doctype entity.name.tag, meta.tag.metadata.doctype punctuation.definition.tag",
            &p.keyword,
            "",
        );
        add("HTML/XML-like <tags/>", "entity.name.tag", &p.func, "");
        add(
            "Special characters like &amp;",
            "text.html constant.character.entity, text.html constant.character.entity punctuation, constant.character.entity.xml, constant.character.entity.xml punctuation, constant.character.entity.js.jsx, constant.charactger.entity.js.jsx punctuation, constant.character.entity.tsx, constant.character.entity.tsx punctuation",
            &p.keyword,
            "",
        );
        add(
            "HTML/XML tag attribute values",
            "entity.other.attribute-name",
            &p.type_name,
            "",
        );
        add(
            "Components",
            "support.class.component, support.class.component.jsx, support.class.component.tsx, support.class.component.vue",
            &p.fg,
            "",
        );
        add(
            "Annotations",
            "punctuation.definition.annotation, storage.type.annotation",
            &p.number,
            "",
        );
        add("Java enums", "constant.other.enum.java", &p.operator, "");
        add("Java imports", "storage.modifier.import.java", &p.fg, "");
        add(
            "Exported Variable",
            "meta.export variable.other.readwrite.js",
            &p.attribute,
            "",
        );
        add(
            "JS/TS constants & properties",
            "variable.other.constant.js, variable.other.constant.ts, variable.other.property.js, variable.other.property.ts",
            &p.fg,
            "",
        );
        add(
            "JSDoc; these are mainly params, so styled as such",
            "variable.other.jsdoc, comment.block.documentation variable.other",
            &p.attribute,
            "",
        );
        add("", "support.type.object.console.js", &p.fg, "");
        add(
            "Node constants as keywords (module, etc.)",
            "support.constant.node, support.type.object.module.js",
            &p.keyword,
            "",
        );
        add(
            "implements as keyword",
            "storage.modifier.implements",
            &p.keyword,
            "",
        );
        add(
            "Builtin types",
            "constant.language.null.js, constant.language.null.ts, constant.language.undefined.js, constant.language.undefined.ts, support.type.builtin.ts",
            &p.keyword,
            "",
        );
        add("", "variable.parameter.generic", &p.type_name, "");
        add(
            "Arrow functions",
            "keyword.declaration.function.arrow.js, storage.type.function.arrow.ts",
            &p.operator,
            "",
        );
        add(
            "Decorator punctuations (decorators inherit from blue functions, instead of styleguide peach)",
            "punctuation.decorator.ts",
            &p.func,
            "italic",
        );
        add(
            "Extra JS/TS keywords",
            "keyword.operator.expression.in.js, keyword.operator.expression.in.ts, keyword.operator.expression.infer.ts, keyword.operator.expression.instanceof.js, keyword.operator.expression.instanceof.ts, keyword.operator.expression.is, keyword.operator.expression.keyof.ts, keyword.operator.expression.of.js, keyword.operator.expression.of.ts, keyword.operator.expression.typeof.ts",
            &p.keyword,
            "",
        );
        add(
            "Julia macros",
            "support.function.macro.julia",
            &p.operator,
            "italic",
        );
        add(
            "Julia language constants (true, false)",
            "constant.language.julia",
            &p.number,
            "",
        );
        add(
            "Julia other constants (these seem to be arguments inside arrays)",
            "constant.other.symbol.julia",
            &p.attribute,
            "",
        );
        add(
            "LaTeX preamble",
            "text.tex keyword.control.preamble",
            &p.operator,
            "",
        );
        add(
            "LaTeX be functions",
            "text.tex support.function.be",
            &p.func,
            "",
        );
        add(
            "LaTeX math",
            "constant.other.general.math.tex",
            &p.attribute,
            "",
        );
        add(
            "Liquid Builtin Objects & User Defined Variables",
            "variable.language.liquid",
            &p.fg,
            "",
        );
        add(
            "Lua docstring keywords",
            "comment.line.double-dash.documentation.lua storage.type.annotation.lua",
            &p.keyword,
            "",
        );
        add(
            "Lua docstring variables",
            "comment.line.double-dash.documentation.lua entity.name.variable.lua, comment.line.double-dash.documentation.lua variable.lua",
            &p.fg,
            "",
        );
        add(
            "",
            "heading.1.markdown punctuation.definition.heading.markdown, heading.1.markdown, heading.1.quarto punctuation.definition.heading.quarto, heading.1.quarto, markup.heading.atx.1.mdx, markup.heading.atx.1.mdx punctuation.definition.heading.mdx, markup.heading.setext.1.markdown, markup.heading.heading-0.asciidoc",
            &p.keyword,
            "",
        );
        add(
            "",
            "heading.2.markdown punctuation.definition.heading.markdown, heading.2.markdown, heading.2.quarto punctuation.definition.heading.quarto, heading.2.quarto, markup.heading.atx.2.mdx, markup.heading.atx.2.mdx punctuation.definition.heading.mdx, markup.heading.setext.2.markdown, markup.heading.heading-1.asciidoc",
            &p.number,
            "",
        );
        add(
            "",
            "heading.3.markdown punctuation.definition.heading.markdown, heading.3.markdown, heading.3.quarto punctuation.definition.heading.quarto, heading.3.quarto, markup.heading.atx.3.mdx, markup.heading.atx.3.mdx punctuation.definition.heading.mdx, markup.heading.heading-2.asciidoc",
            &p.type_name,
            "",
        );
        add(
            "",
            "heading.4.markdown punctuation.definition.heading.markdown, heading.4.markdown, heading.4.quarto punctuation.definition.heading.quarto, heading.4.quarto, markup.heading.atx.4.mdx, markup.heading.atx.4.mdx punctuation.definition.heading.mdx, markup.heading.heading-3.asciidoc",
            &p.string,
            "",
        );
        add(
            "",
            "heading.5.markdown punctuation.definition.heading.markdown, heading.5.markdown, heading.5.quarto punctuation.definition.heading.quarto, heading.5.quarto, markup.heading.atx.5.mdx, markup.heading.atx.5.mdx punctuation.definition.heading.mdx, markup.heading.heading-4.asciidoc",
            &p.func,
            "",
        );
        add(
            "",
            "heading.6.markdown punctuation.definition.heading.markdown, heading.6.markdown, heading.6.quarto punctuation.definition.heading.quarto, heading.6.quarto, markup.heading.atx.6.mdx, markup.heading.atx.6.mdx punctuation.definition.heading.mdx, markup.heading.heading-5.asciidoc",
            &p.variable,
            "",
        );
        add("", "markup.bold", &p.keyword, "bold");
        add("", "markup.italic", &p.keyword, "italic");
        add("", "markup.strikethrough", &p.fg, "strikethrough");
        add(
            "Markdown auto links",
            "punctuation.definition.link, markup.underline.link",
            &p.func,
            "",
        );
        add(
            "Markdown links",
            "text.html.markdown punctuation.definition.link.title, text.html.quarto punctuation.definition.link.title, string.other.link.title.markdown, string.other.link.title.quarto, markup.link, punctuation.definition.constant.markdown, punctuation.definition.constant.quarto, constant.other.reference.link.markdown, constant.other.reference.link.quarto, markup.substitution.attribute-reference",
            &p.variable,
            "",
        );
        add(
            "Markdown code spans",
            "punctuation.definition.raw.markdown, punctuation.definition.raw.quarto, markup.inline.raw.string.markdown, markup.inline.raw.string.quarto, markup.raw.block.markdown, markup.raw.block.quarto",
            &p.string,
            "",
        );
        add(
            "Markdown triple backtick language identifier",
            "fenced_code.block.language",
            &p.func,
            "",
        );
        add(
            "Markdown triple backticks",
            "markup.fenced_code.block punctuation.definition, markup.raw support.asciidoc",
            &p.comment,
            "",
        );
        add(
            "Markdown quotes",
            "markup.quote, punctuation.definition.quote.begin",
            &p.fg,
            "",
        );
        add(
            "Markdown separators",
            "meta.separator.markdown",
            &p.operator,
            "",
        );
        add(
            "Markdown list bullets",
            "punctuation.definition.list.begin.markdown, punctuation.definition.list.begin.quarto, markup.list.bullet",
            &p.operator,
            "",
        );
        add(
            "Nix attribute names",
            "entity.other.attribute-name.multipart.nix, entity.other.attribute-name.single.nix",
            &p.func,
            "",
        );
        add(
            "Nix parameter names",
            "variable.parameter.name.nix",
            &p.fg,
            "",
        );
        add(
            "Nix interpolated parameter names",
            "meta.embedded variable.parameter.name.nix",
            &p.variable,
            "",
        );
        add("Nix paths", "string.unquoted.path.nix", &p.fg, "");
        add(
            "PHP Attributes",
            "support.attribute.builtin, meta.attribute.php",
            &p.type_name,
            "",
        );
        add(
            "PHP Parameters (needed for the leading dollar sign)",
            "meta.function.parameters.php punctuation.definition.variable.php",
            &p.attribute,
            "",
        );
        add(
            "PHP Constants (null, __FILE__, etc.)",
            "constant.language.php",
            &p.keyword,
            "",
        );
        add(
            "PHP functions",
            "text.html.php support.function",
            &p.func,
            "",
        );
        add(
            "Python argument functions reset to text, otherwise they inherit blue from function-call",
            "support.variable.magic.python, meta.function-call.arguments.python",
            &p.fg,
            "",
        );
        add(
            "Python double underscore functions",
            "support.function.magic.python",
            &p.func,
            "italic",
        );
        add(
            "Python `self` keyword",
            "variable.parameter.function.language.special.self.python, variable.language.special.self.python",
            &p.keyword,
            "italic",
        );
        add(
            "python keyword flow/logical (for ... in)",
            "keyword.control.flow.python, keyword.operator.logical.python",
            &p.keyword,
            "",
        );
        add(
            "python storage type",
            "storage.type.function.python",
            &p.keyword,
            "",
        );
        add(
            "python function support",
            "support.token.decorator.python, meta.function.decorator.identifier.python",
            &p.func,
            "",
        );
        add(
            "python function calls",
            "meta.function-call.python",
            &p.func,
            "",
        );
        add(
            "python function decorators",
            "entity.name.function.decorator.python, punctuation.definition.decorator.python",
            &p.number,
            "italic",
        );
        add(
            "python placeholder reset to normal string",
            "constant.character.format.placeholder.other.python",
            &p.fg,
            "",
        );
        add(
            "Python exception & builtins such as exit()",
            "support.type.exception.python, support.function.builtin.python",
            &p.number,
            "",
        );
        add("entity.name.type", "support.type.python", &p.keyword, "");
        add(
            "python constants (True/False)",
            "constant.language.python",
            &p.number,
            "",
        );
        add(
            "Arguments accessed later in the function body",
            "meta.indexed-name.python, meta.item-access.python",
            &p.attribute,
            "italic",
        );
        add(
            "Python f-strings/binary/unicode storage types",
            "storage.type.string.python",
            &p.string,
            "italic",
        );
        add(
            "Regex string begin/end in JS/TS",
            "string.regexp punctuation.definition.string.begin, string.regexp punctuation.definition.string.end",
            &p.fg,
            "",
        );
        add(
            "Regex anchors (^, $)",
            "keyword.control.anchor.regexp",
            &p.keyword,
            "",
        );
        add("Regex regular string match", "string.regexp.ts", &p.fg, "");
        add(
            "Regex group parenthesis & backreference",
            "punctuation.definition.group.regexp, keyword.other.back-reference.regexp",
            &p.string,
            "",
        );
        add(
            "Regex character class []",
            "punctuation.definition.character-class.regexp",
            &p.type_name,
            "",
        );
        add(
            "Regex character classes",
            "constant.other.character-class.regexp",
            &p.fg,
            "",
        );
        add(
            "Regex range",
            "constant.other.character-class.range.regexp",
            &p.attribute,
            "",
        );
        add(
            "Regex quantifier",
            "keyword.operator.quantifier.regexp",
            &p.operator,
            "",
        );
        add(
            "Regex constant/numeric",
            "constant.character.numeric.regexp",
            &p.number,
            "",
        );
        add(
            "Regex lookaheads, negative lookaheads, lookbehinds, negative lookbehinds",
            "punctuation.definition.group.no-capture.regexp, meta.assertion.look-ahead.regexp, meta.assertion.negative-look-ahead.regexp",
            &p.func,
            "",
        );
        add(
            "Rust attribute",
            "meta.annotation.rust, meta.annotation.rust punctuation, meta.attribute.rust, punctuation.definition.attribute.rust",
            &p.type_name,
            "italic",
        );
        add(
            "Rust keyword",
            "entity.name.function.macro.rules.rust, storage.type.module.rust, storage.modifier.rust, storage.type.struct.rust, storage.type.enum.rust, storage.type.trait.rust, storage.type.union.rust, storage.type.impl.rust, storage.type.rust, storage.type.function.rust, storage.type.type.rust",
            &p.keyword,
            "",
        );
        add(
            "Rust u/i32, u/i64, etc.",
            "entity.name.type.numeric.rust",
            &p.keyword,
            "",
        );
        add("Rust generic", "meta.generic.rust", &p.number, "");
        add("Rust impl", "entity.name.impl.rust", &p.type_name, "italic");
        add("Rust module", "entity.name.module.rust", &p.number, "");
        add(
            "Rust trait",
            "entity.name.trait.rust",
            &p.type_name,
            "italic",
        );
        add("Rust struct", "storage.type.source.rust", &p.type_name, "");
        add("Rust union", "entity.name.union.rust", &p.type_name, "");
        add(
            "Rust enum member",
            "meta.enum.rust storage.type.source.rust",
            &p.operator,
            "",
        );
        add(
            "Rust macro",
            "support.macro.rust, meta.macro.rust support.function.rust, entity.name.function.macro.rust",
            &p.func,
            "italic",
        );
        add(
            "Rust lifetime",
            "storage.modifier.lifetime.rust, entity.name.type.lifetime",
            &p.func,
            "italic",
        );
        add(
            "Rust string formatting",
            "string.quoted.double.rust constant.other.placeholder.rust",
            &p.fg,
            "",
        );
        add(
            "Rust return type generic",
            "meta.function.return-type.rust meta.generic.rust storage.type.rust",
            &p.fg,
            "",
        );
        add("Rust functions", "meta.function.call.rust", &p.func, "");
        add(
            "Rust angle brackets",
            "punctuation.brackets.angle.rust",
            &p.func,
            "",
        );
        add("Rust constants", "constant.other.caps.rust", &p.number, "");
        add(
            "Rust function parameters",
            "meta.function.definition.rust variable.other.rust",
            &p.attribute,
            "",
        );
        add(
            "Rust closure variables",
            "meta.function.call.rust variable.other.rust",
            &p.fg,
            "",
        );
        add("Rust self", "variable.language.self.rust", &p.keyword, "");
        add(
            "Rust metavariable names",
            "variable.other.metavariable.name.rust, meta.macro.metavariable.rust keyword.operator.macro.dollar.rust",
            &p.fg,
            "",
        );
        add(
            "Shell shebang",
            "comment.line.shebang, comment.line.shebang punctuation.definition.comment, comment.line.shebang, punctuation.definition.comment.shebang.shell, meta.shebang.shell",
            &p.fg,
            "italic",
        );
        add(
            "Shell shebang command",
            "comment.line.shebang constant.language",
            &p.operator,
            "italic",
        );
        add(
            "Shell interpolated command",
            "meta.function-call.arguments.shell punctuation.definition.variable.shell, meta.function-call.arguments.shell punctuation.section.interpolation, meta.function-call.arguments.shell punctuation.definition.variable.shell, meta.function-call.arguments.shell punctuation.section.interpolation",
            &p.keyword,
            "",
        );
        add(
            "Shell interpolated command variable",
            "meta.string meta.interpolation.parameter.shell variable.other.readwrite",
            &p.number,
            "italic",
        );
        add(
            "",
            "source.shell punctuation.section.interpolation, punctuation.definition.evaluation.backticks.shell",
            &p.operator,
            "",
        );
        add("Shell EOF", "entity.name.tag.heredoc.shell", &p.keyword, "");
        add(
            "Shell quoted variable",
            "string.quoted.double.shell variable.other.normal.shell",
            &p.fg,
            "",
        );
        add("", "markup.heading.typst", &p.keyword, "");
        add(
            "JSON Keys",
            "source.json meta.mapping.key string",
            &p.func,
            "",
        );
        add(
            "JSON key surrounding quotes",
            "source.json meta.mapping.key punctuation.definition.string.begin, source.json meta.mapping.key punctuation.definition.string.end",
            &p.comment,
            "",
        );
        add(
            "",
            "markup.heading.synopsis.man, markup.heading.title.man, markup.heading.other.man, markup.heading.env.man",
            &p.keyword,
            "",
        );
        add("", "markup.heading.commands.man", &p.func, "");
        add("", "markup.heading.env.man", &p.fg, "");
        add("Man page options", "entity.name", &p.operator, "");
        add("", "markup.heading.1.markdown", &p.keyword, "");
        add("", "markup.heading.2.markdown", &p.number, "");
        add("", "markup.heading.markdown", &p.type_name, "");

        s
    }
}
