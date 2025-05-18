import os
import shutil


def on_post_build(config, **kwargs):
    site_dir = config["site_dir"]
    shutil.copytree("pkg", os.path.join(site_dir, "dist"), dirs_exist_ok=True)
