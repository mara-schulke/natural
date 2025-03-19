import os
import sys
import subprocess

__venv = None


def use_virtualenv(venv_path):
    """Manually configure sys.path to use a virtual environment."""

    if not os.path.exists(venv_path):
        print(f"Error: Virtual environment not found at {venv_path}")
        return False

    os.environ["VIRTUAL_ENV"] = venv_path

    bin_folder = "Scripts" if os.name == "nt" else "bin"
    os.environ["_OLD_VIRTUAL_PATH"] = os.environ.get("PATH", "")
    os.environ["PATH"] = (
        os.path.join(venv_path, bin_folder)
        + os.pathsep
        + os.environ["_OLD_VIRTUAL_PATH"]
    )

    if "PYTHONHOME" in os.environ:
        os.environ["_OLD_VIRTUAL_PYTHONHOME"] = os.environ["PYTHONHOME"]
        del os.environ["PYTHONHOME"]

    python_version = f"python{sys.version_info.major}.{sys.version_info.minor}"

    site_packages = (
        os.path.join(venv_path, "lib", python_version, "site-packages")
        if os.name != "nt"
        else os.path.join(venv_path, "Lib", "site-packages")
    )

    if not os.path.exists(site_packages):
        print(f"Error: Virtual environment site packages not found at {site_packages}")
        return False

    sys.path.insert(0, site_packages)

    os.environ["_OLD_VIRTUAL_PS1"] = os.environ.get("PS1", "")
    os.environ["VIRTUAL_ENV_PROMPT"] = f"({os.path.basename(venv_path)}) "

    print(f"Activated virtual environment: {venv_path}")

    return True


def activate_venv(venv):
    global __venv
    if __venv == venv:
        return True

    if sys.platform in ("win32", "win64", "cygwin"):
        activate = os.path.join(venv, "Scripts", "activate")
    else:
        activate = os.path.join(venv, "bin", "activate")

    if os.path.exists(activate):
        subprocess.run(f"source {activate}", shell=True, executable="/bin/bash")
        __venv = venv
        return True
    else:
        print("virtualenv not found: %s" % venv, file=sys.stderr)
        return False


def freeze():
    try:
        from pip._internal.operations import freeze
    except ImportError:  # pip < 10.0
        from pip.operations import freeze

    return list(freeze.freeze())
