"""Self-owned silent audio / artwork / lyrics. Writes only target/lyrics-ui."""
from pathlib import Path
import hashlib
import json
import wave
from PIL import Image, ImageDraw

root = Path(__file__).resolve().parent.parent / "target" / "lyrics-ui"
root.mkdir(parents=True, exist_ok=True)
media = root / "media"
media.mkdir(parents=True, exist_ok=True)
for name in ("coral", "blue", "no-cover", "plain"):
    with wave.open(str(media / f"{name}.wav"), "wb") as audio:
        audio.setnchannels(2)
        audio.setsampwidth(2)
        audio.setframerate(44100)
        audio.writeframes(bytes(44100 * 4 * 65))

for name, color in (("coral", (198, 68, 90)), ("blue", (40, 104, 192)), ("plain", (36, 124, 112))):
    art = Image.new("RGB", (512, 512), color)
    draw = ImageDraw.Draw(art)
    draw.ellipse((270, 55, 442, 227), fill=(242, 219, 184))
    for i in range(6):
        draw.arc((-130, 250 + i * 24, 630, 850 + i * 24), 175, 345, fill=(240, 192, 169), width=3)
    draw.text((44, 44), "YYPLAYER / SESSIONS", fill=(253, 235, 219))
    art.save(media / f"{name}.png")

phrases = [
    "Listen to the evening / 听见晚风",
    "A quiet place for your sound / 为声音留一片宁静",
    "Follow the light / 沿着光的方向",
    "This is a longer line that wraps naturally across the lyric column without losing its translation / 这是一句较长的歌词，换行以后仍然保留上下两种语言的间距",
    "夜の風に耳を澄まして / 静静聆听夜晚的风",
    "The night unfolds",
    "Keep the moment close / 把此刻留在身边",
]
lyrics = "\n".join(f"[00:{i * 3:02}.00]{phrases[i % len(phrases)]}" for i in range(20))
for name in ("coral", "blue"):
    (media / f"{name}.lrc").write_text(lyrics + "\n", encoding="utf-8")
(media / "plain.lrc").write_text("Untimed lyrics / 没有时间轴的歌词\nTwo languages, two lines / 两种语言，上下显示\nAC/DC keeps its slash\n", encoding="utf-8")
source_names = {f"{name}.{ext}" for name in ("coral", "blue", "no-cover", "plain") for ext in ("wav", "png", "lrc")}
hashes = {p.relative_to(root).as_posix(): hashlib.sha256(p.read_bytes()).hexdigest() for p in sorted(media.iterdir()) if p.name in source_names}
(root / "fixtures.json").write_text(json.dumps(hashes, indent=2), encoding="utf-8")
print(f"Prepared self-owned lyric fixtures in {root}")
