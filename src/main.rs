use std::io::{self, Write};
use std::path::PathBuf;
use std::fs;
use anyhow::Result;

fn prompt(label: &str) -> Result<String> {
    print!("{}: ", label);
    io::stdout().flush()?;
    let mut s = String::new();
    io::stdin().read_line(&mut s)?;
    Ok(s.trim().to_string())
}

fn main() -> Result<()> {
    let input = loop {
        let p = prompt("Music file path")?;
        if p.is_empty() {
            println!("File path cannot be empty");
            continue;
        }
        let pb = PathBuf::from(&p);
        if pb.exists() {
            break pb;
        } else {
            println!("File not found, try again");
        }
    };

    let title = prompt("Track title")?;
    let artist = prompt("Artist name")?;
    let album = prompt("Album name")?;
    let picture_input = prompt("Cover image path (leave empty to skip)")?;
    let output_input = prompt("Output path (leave empty to save next to original file)")?;

    let ext = input.extension().and_then(|s| s.to_str()).unwrap_or("").to_lowercase();

    let output = if output_input.is_empty() {
        let mut p = input.clone();
        let stem = p.file_stem().and_then(|s| s.to_str()).unwrap_or("output");
        let new_name = format!("{}-edited.{}", stem, ext);
        p.set_file_name(new_name);
        p
    } else {
        PathBuf::from(output_input)
    };

    fs::copy(&input, &output)?;

    let picture = if picture_input.is_empty() {
        None
    } else {
        Some(PathBuf::from(picture_input))
    };

    if ext == "mp3" {
        handle_mp3(&output, title, artist, album, picture)?;
    } else if ext == "opus" {
        handle_opus(&output, title, artist, album, picture)?;
    } else {
        println!("Unsupported file type: {}", ext);
    }

    println!("Saved to: {}", output.display());
    Ok(())
}

fn handle_mp3(
    path: &PathBuf,
    title: String,
    artist: String,
    album: String,
    picture: Option<PathBuf>,
) -> Result<()> {
    use id3::{Tag, Version, TagLike, frame::Picture, frame::PictureType};

    let mut tag = Tag::read_from_path(path).unwrap_or_else(|_| Tag::new());

    if !title.is_empty() {
        tag.set_title(title);
    }
    if !artist.is_empty() {
        tag.set_artist(artist);
    }
    if !album.is_empty() {
        tag.set_album(album);
    }

    if let Some(pic) = picture {
        let data = fs::read(&pic)?;
        let mime = match pic.extension().and_then(|s| s.to_str()).unwrap_or("").to_lowercase().as_str() {
            "png" => "image/png",
            "jpg" | "jpeg" => "image/jpeg",
            _ => "image/jpeg",
        }.to_string();
        tag.add_frame(Picture {
            mime_type: mime,
            picture_type: PictureType::CoverFront,
            description: String::new(),
                      data,
        });
    }

    tag.write_to_path(path, Version::Id3v24)?;
    Ok(())
}

fn handle_opus(
    path: &PathBuf,
    title: String,
    artist: String,
    album: String,
    picture: Option<PathBuf>,
) -> Result<()> {
    use lofty::probe::Probe;
    use lofty::prelude::{AudioFile, ItemKey, TaggedFileExt};
    use lofty::tag::{Tag as LoftyTag, TagType};
    use lofty::picture::{MimeType, Picture, PictureType};
    use lofty::config::WriteOptions;

    let mut tagged = Probe::open(path)?.read()?;

    if tagged.primary_tag().is_none() {
        tagged.insert_tag(LoftyTag::new(TagType::VorbisComments));
    }

    let tag = tagged.primary_tag_mut().unwrap();

    if !title.is_empty() {
        tag.insert_text(ItemKey::TrackTitle, title);
    }
    if !artist.is_empty() {
        tag.insert_text(ItemKey::TrackArtist, artist);
    }
    if !album.is_empty() {
        tag.insert_text(ItemKey::AlbumTitle, album);
    }

    if let Some(pic) = picture {
        let data = fs::read(&pic)?;
        let mime = match pic.extension().and_then(|s| s.to_str()).unwrap_or("").to_lowercase().as_str() {
            "png" => MimeType::Png,
            _ => MimeType::Jpeg,
        };
        tag.push_picture(Picture::new_unchecked(
            PictureType::CoverFront,
            Some(mime),
                                                None,
                                                data,
        ));
    }

    tagged.save_to_path(path, WriteOptions::default())?;
    Ok(())
}
