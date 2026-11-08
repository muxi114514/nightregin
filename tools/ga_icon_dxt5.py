"""把一张 DDS 小图转成 DXT5（黑夜君临的 UI 图集是 DXT5，GA 的是 BC7）：ga_icon_dxt5.py <输入.dds> <输出.dds>，由 tools\\ga_port.ps1 调用"""
import sys

from PIL import Image

Image.open(sys.argv[1]).convert("RGBA").save(sys.argv[2], pixel_format="DXT5")
