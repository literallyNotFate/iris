use crate::{core::IrisEngine, log::Activity, modules::Generator};
use anyhow::{Context, Result};
use std::path::PathBuf;

/// Strategy for applying a theme to a specific application
#[derive(Clone)]
pub enum Strategy {
    /// Generates file in .cache/iris and makes symlink into app config
    Symlink,

    /// Fully overwrites static config file
    StaticOverwrite,

    /// Makes step in order to apply theme
    Pipeline { steps: Vec<PipelineStep> },

    /// Injects necessary settings into existing config file
    InjectBlock { file: String },
}

/// Step for a pipeline strategy
#[derive(Clone, Debug)]
pub enum PipelineStep {
    /// Generate file from template in specified path (e.g. in .cache or .config)
    GenerateFile {
        template_name: String,
        destination: PathBuf,
    },
    /// Inject or update block of code in configuration file
    InjectBlock {
        file_path: PathBuf,
        marker: String,
        content: String,
    },
    /// Run external system command (e.g. `bat cache --build` or `source ~/.zshrc`)
    RunCommand {
        program: String,
        args: Vec<String>,
        silent: bool,
    },
}

impl Strategy {
    /// Dispatches execution to the specific system mutation step.
    /// Internal errors are contextualized locally, while IrisEngine handles global rollbacks
    pub fn apply<G: Generator + ?Sized>(
        &self,
        engine: &IrisEngine,
        generator: &G,
        cache_path: &PathBuf,
        link_path: &PathBuf,
        log: &mut Activity,
    ) -> Result<()> {
        use colored::*;

        match self {
            Self::Symlink => {
                log.info(&format!(
                    "Linking {} theme to {}...",
                    generator.name().bold().cyan(),
                    crate::utils::pretty_path(link_path).magenta(),
                ));

                engine.atomic_symlink(cache_path, link_path)?;
            }

            Self::StaticOverwrite => {
                log.info(&format!(
                    "Writing static config for {} to {}...",
                    generator.name().bold().cyan(),
                    crate::utils::pretty_path(link_path).magenta(),
                ));

                engine.atomic_write(cache_path, link_path)?;
            }

            Self::Pipeline { .. } => {
                log.info(&format!(
                    "Executing pipeline for {}...",
                    generator.name().bold().cyan()
                ));

                let steps: Vec<PipelineStep> = generator.pipeline_steps(engine.paths, engine.theme);
                for step in steps {
                    match step {
                        PipelineStep::GenerateFile {
                            template_name: _,
                            destination,
                        } => {
                            log.info(&format!(
                                "Generating cache file for {}...",
                                generator.name().bold().cyan()
                            ));
                            engine.atomic_write(cache_path, &destination)?;
                        }
                        PipelineStep::InjectBlock {
                            file_path,
                            marker,
                            content,
                        } => {
                            log.info(&format!(
                                "Injecting block in {}",
                                crate::utils::pretty_path(&file_path).magenta()
                            ));
                            engine.inject_block(&file_path, &marker, &content)?;
                        }
                        PipelineStep::RunCommand {
                            program,
                            args,
                            silent,
                        } => {
                            log.info(&format!(
                                "Running: {} {}",
                                program.green(),
                                args.join(" ").bright_green()
                            ));

                            let mut cmd = std::process::Command::new(&program);
                            cmd.args(&args);

                            if silent {
                                cmd.stdout(std::process::Stdio::null());
                                cmd.stderr(std::process::Stdio::null());
                            }

                            let status = cmd.status().with_context(|| {
                                format!("Failed to execute command: `{}`", program)
                            })?;

                            if !status.success() {
                                anyhow::bail!(
                                    "Command `{}` failed with status: {}",
                                    program,
                                    status
                                );
                            }
                        }
                    }
                }
            }

            Self::InjectBlock { file } => {
                log.info(&format!(
                    "Injecting theme/palette block into {}...",
                    file.bold().magenta()
                ));

                let content: String = std::fs::read_to_string(cache_path)?;
                engine.inject_block(link_path, generator.name(), &content)?;
            }
        }

        Ok(())
    }
}
