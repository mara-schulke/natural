from __future__ import annotations

import os
import site
import sys

__venv = None


def activate_venv(venv):
    global __venv
    if __venv == venv:
        return True

    # Ensure venv path is absolute
    venv = os.path.abspath(venv)

    if sys.platform in ("win32", "win64", "cygwin"):
        activate_this = os.path.join(venv, "Scripts", "activate_this.py")
        site_packages = os.path.join(venv, "Lib", "site-packages")
    else:
        activate_this = os.path.join(venv, "bin", "activate_this.py")
        py_version = f"python{sys.version_info.major}.{sys.version_info.minor}"
        site_packages = os.path.join(venv, "lib", py_version, "site-packages")

    if os.path.exists(activate_this):
        info(f"activating via {activate_this}")

        # Add site-packages to sys.path first (before using activate_this)
        if os.path.exists(site_packages) and site_packages not in sys.path:
            sys.path.insert(0, site_packages)
            # Also update site.addsitedir to properly handle .pth files
            site.addsitedir(site_packages)
            info(f"Added site-packages: {site_packages}")

        # Run activate_this.py script
        exec(open(activate_this).read(), dict(__file__=activate_this))

        # Make sure the current directory is not in sys.path (to avoid importing from source)
        if "" in sys.path:
            sys.path.remove("")
        if "." in sys.path:
            sys.path.remove(".")

        __venv = venv
        info(f"Virtual environment activated: {venv}")
        return True
    else:
        print(f"virtualenv not found: {venv}", file=sys.stderr)
        return False


def freeze():
    try:
        from pip._internal.operations import freeze
    except ImportError:
        from pip.operations import freeze
    return list(freeze.freeze())


def get_sys_path():
    """Return the current sys.path for debugging"""
    return sys.path
