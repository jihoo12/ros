from PIL import Image

# 이미지 열기 및 리사이즈
img = Image.open("wakamo1.jpg").resize((500, 500)).convert("RGBA")

# RGBA 데이터를 가져와서 B와 R의 위치를 바꿉니다.
# UEFI 환경은 보통 BGRA(Little Endian u32로는 0xAARRGGBB)를 선호합니다.
r, g, b, a = img.split()
img_bgra = Image.merge("RGBA", (b, g, r, a)) # R과 B의 위치를 스왑

with open("image.bin", "wb") as f:
    f.write(img_bgra.tobytes())