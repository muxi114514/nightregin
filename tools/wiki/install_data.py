"""把生成好的图鉴整体换进整合包。用法：python -I install_data.py <生成目录> <launcher\\data>"""
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from wiki_lib import install_dir  # noqa: E402

install_dir(sys.argv[1], sys.argv[2])
