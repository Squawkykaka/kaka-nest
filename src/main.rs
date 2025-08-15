mod get_markdown;

fn main() -> color_eyre::eyre::Result<()> {
    get_markdown::get_blogs()?;

    Ok(())
}
