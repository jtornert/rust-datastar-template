use std::path::Path;

use glob::glob;

fn build_templates() {
    let templates_root = Path::new("dist").join("templates");
    if !templates_root.exists() {
        std::fs::create_dir_all(&templates_root).unwrap();
    }
    for entry in glob("src/resources/**/*.j2").unwrap() {
        match entry {
            Ok(path) => {
                let output = templates_root.join(path.strip_prefix("src/resources/").unwrap());
                let output_parent = output.parent().unwrap();
                if !output_parent.exists() {
                    std::fs::create_dir_all(output_parent).unwrap();
                }
                let content = std::fs::read(path).unwrap();
                let mut cfg = minify_html::Cfg::new();
                cfg.keep_closing_tags = true;
                cfg.keep_html_and_head_opening_tags = true;
                cfg.remove_bangs = true;
                let minified = minify_html::minify(&content, &cfg);
                std::fs::write(output, minified).unwrap();
            }
            _ => continue,
        }
    }
}

fn build_global_styles() {
    let input = Path::new("src")
        .join("assets")
        .join("css")
        .join("global.css");
    if !input.exists() {
        return;
    }
    let css_root = Path::new("dist").join("assets").join("css");
    if !css_root.exists() {
        std::fs::create_dir_all(&css_root).unwrap();
    }
    let output = css_root.join("global.css");
    if output.exists() {
        let (Ok(input_modified), Ok(output_modified)) = (
            input.metadata().and_then(|m| m.modified()),
            output.metadata().and_then(|m| m.modified()),
        ) else {
            return;
        };
        if input_modified < output_modified {
            return;
        }
    }
    let file_provider = lightningcss::bundler::FileProvider::new();
    let mut bundler = lightningcss::bundler::Bundler::new(
        &file_provider,
        None,
        lightningcss::stylesheet::ParserOptions::default(),
    );
    let mut stylesheet = bundler.bundle(&input).unwrap();
    stylesheet
        .minify(lightningcss::stylesheet::MinifyOptions::default())
        .unwrap();
    let result = stylesheet
        .to_css(lightningcss::printer::PrinterOptions {
            minify: true,
            ..Default::default()
        })
        .unwrap();
    std::fs::write(output, result.code).unwrap();
}

fn build_page_styles() {
    let templates_root = Path::new("dist").join("assets").join("pages");
    let file_provider = lightningcss::bundler::FileProvider::new();
    let mut bundler = lightningcss::bundler::Bundler::new(
        &file_provider,
        None,
        lightningcss::stylesheet::ParserOptions::default(),
    );
    for entry in glob("src/resources/**/*.css").unwrap() {
        match entry {
            Ok(input) => {
                let output = templates_root.join(input.strip_prefix("src/resources/").unwrap());
                let output_parent = output.parent().unwrap();
                if !output_parent.exists() {
                    std::fs::create_dir_all(output_parent).unwrap();
                }
                if output.exists() {
                    let (Ok(input_modified), Ok(output_modified)) = (
                        input.metadata().and_then(|m| m.modified()),
                        output.metadata().and_then(|m| m.modified()),
                    ) else {
                        return;
                    };
                    if input_modified < output_modified {
                        return;
                    }
                }
                let mut stylesheet = bundler.bundle(&input).unwrap();
                stylesheet
                    .minify(lightningcss::stylesheet::MinifyOptions::default())
                    .unwrap();
                let result = stylesheet
                    .to_css(lightningcss::printer::PrinterOptions {
                        minify: true,
                        ..Default::default()
                    })
                    .unwrap();
                std::fs::write(output, result.code).unwrap();
            }
            _ => continue,
        }
    }
}

async fn build_global_scripts() {
    let js_root = Path::new("dist").join("assets").join("js");
    if !js_root.exists() {
        std::fs::create_dir_all(&js_root).unwrap();
    }
    for entry in glob("src/assets/ts/**/*.ts").unwrap() {
        match entry {
            Ok(path) => {
                let mut options_builder = esbuild_rs::BuildOptionsBuilder::new();
                options_builder
                    .entry_points
                    .push(path.to_str().unwrap().into());
                options_builder.bundle = true;
                options_builder.format = esbuild_rs::Format::ESModule;
                options_builder.minify_identifiers = true;
                options_builder.minify_syntax = true;
                options_builder.minify_whitespace = true;
                let result = esbuild_rs::build(options_builder.build())
                    .await
                    .output_files;
                let content = result.as_slice().first().unwrap().data.as_str().to_owned();
                let output = js_root.join(
                    path.strip_prefix("src/assets/ts")
                        .unwrap()
                        .with_extension("js"),
                );
                std::fs::write(output, content).unwrap();
            }
            _ => continue,
        }
    }
}

async fn build_page_scripts() {
    let js_root = Path::new("dist").join("assets").join("js");
    if !js_root.exists() {
        std::fs::create_dir_all(&js_root).unwrap();
    }
    for entry in glob("src/resources/**/*.ts").unwrap() {
        match entry {
            Ok(input) => {
                let output = Path::new("dist")
                    .join("assets")
                    .join("pages")
                    .join(input.strip_prefix("src/resources").unwrap())
                    .with_extension("js");
                let output_parent = output.parent().unwrap();
                if !output_parent.exists() {
                    std::fs::create_dir_all(output_parent).unwrap();
                }
                let mut options_builder = esbuild_rs::BuildOptionsBuilder::new();
                options_builder
                    .entry_points
                    .push(input.to_str().unwrap().into());
                options_builder.bundle = true;
                options_builder.format = esbuild_rs::Format::ESModule;
                options_builder.minify_identifiers = true;
                options_builder.minify_syntax = true;
                options_builder.minify_whitespace = true;
                let result = esbuild_rs::build(options_builder.build())
                    .await
                    .output_files;
                let content = result.as_slice().first().unwrap().data.as_str().to_owned();
                std::fs::write(output, content).unwrap();
            }
            _ => continue,
        }
    }
}

fn main() {
    if std::env::var("PROFILE").is_ok_and(|p| p == "release") {
        std::fs::remove_dir_all("dist").unwrap();
        build_templates();
        build_global_styles();
        build_page_styles();
        tokio::runtime::Builder::new_current_thread()
            .build()
            .unwrap()
            .block_on(async { tokio::join!(build_global_scripts(), build_page_scripts()) });
    }
}
