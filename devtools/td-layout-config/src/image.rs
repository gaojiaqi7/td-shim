use serde::Deserialize;

use crate::{config::LayoutConfig, render};

#[derive(Deserialize, Debug, PartialEq)]
struct ImageConfig {
    #[serde(rename = "Config")]
    config: String,
    #[serde(rename = "Mailbox")]
    mailbox: String,
    #[serde(rename = "TempStack")]
    temp_stack: String,
    #[serde(rename = "TempHeap")]
    temp_heap: String,
    #[serde(rename = "BuiltinPayload")]
    builtin_payload: Option<String>,
    #[serde(rename = "Metadata")]
    metadata: String,
    #[serde(rename = "Bootloader")]
    bootloader: String,
    #[serde(rename = "ResetVector")]
    reset_vector: String,
}

pub fn parse_image(data: String) -> String {
    let image_config = serde_json::from_str::<ImageConfig>(&data)
        .expect("Content is configuration file is invalid");

    let mut image_layout = LayoutConfig::new(0, 0x100_0000);
    image_layout.alloc_low(
        "Config",
        parse_int::parse::<u32>(&image_config.config).unwrap() as usize,
        "Reserved",
    );
    image_layout.alloc_low(
        "Mailbox",
        parse_int::parse::<u32>(&image_config.mailbox).unwrap() as usize,
        "Reserved",
    );
    image_layout.alloc_low(
        "TempStack",
        parse_int::parse::<u32>(&image_config.temp_stack).unwrap() as usize,
        "Reserved",
    );
    image_layout.alloc_low(
        "TempHeap",
        parse_int::parse::<u32>(&image_config.temp_heap).unwrap() as usize,
        "Reserved",
    );

    image_layout.alloc_high(
        "ResetVector",
        parse_int::parse::<u32>(&image_config.reset_vector).unwrap() as usize,
        "Reserved",
    );
    image_layout.alloc_high(
        "Bootloader",
        parse_int::parse::<u32>(&image_config.bootloader).unwrap() as usize,
        "Reserved",
    );

    image_layout.alloc_high(
        "Metadata",
        parse_int::parse::<u32>(&image_config.metadata).unwrap() as usize,
        "Reserved",
    );

    if let Some(payload_config) = image_config.builtin_payload {
        image_layout.alloc_high(
            "BuiltinPayload",
            parse_int::parse::<u32>(&payload_config).unwrap() as usize,
            "Reserved",
        )
    }

    render::render_image(&image_layout).expect("Render image layout failed!")
}
