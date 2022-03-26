#     ____                  __        ____
#    / __ \____ ___________/ /_____ _/ / /
#   / /_/ / __ `/ ___/ ___/ __/ __ `/ / /
#  / ____/ /_/ / /__(__  ) /_/ /_/ / / /
# /_/    \__,_/\___/____/\__/\__,_/_/_/
#
# Copyright (C) 2022-present
#
# This file is part of Pacstall
#
# Pacstall is free software: you can redistribute it and/or modify
# it under the terms of the GNU General Public License as published by
# the Free Software Foundation, version 3 of the License
#
# Pacstall is distributed in the hope that it will be useful,
# but WITHOUT ANY WARRANTY; without even the implied warranty of
# MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
# GNU General Public License for more details.
#
# You should have received a copy of the GNU General Public License
# along with Pacstall. If not, see <https://www.gnu.org/licenses/>.

"""Module for config parsing."""

from os import cpu_count, environ
from pathlib import Path
from typing import Any, Dict

from tomli import load


class Settings:
    """
    Facade for the file settings.

    Attributes
    ----------
    jobs
        The number of jobs to use for building.
    editor
        The editor to use for opening files.
    """

    jobs: int
    editor: str

    def __init__(self, raw_config: Dict[str, Any]) -> None:
        """
        Initialize the settings.

        Parameters
        ----------
        raw_config
            The raw config dictionary.
        """

        settings = raw_config["settings"]
        self.jobs = settings.get("jobs", cpu_count())

        # Loading order:
        # 1. `editor` config file value.
        # 2. `EDITOR` environment variable.
        # 3. `VISUAL` environment variable.
        # 4. `sensible-editor`
        self.editor = settings.get(
            "editor", environ.get("EDITOR", environ.get("VISUAL", "sensible-editor"))
        )


class Config:
    """
    Facade for the config file.

    Attributes
    ----------
    raw_config
        The raw config parsed dictionary.
    settings
        Facade for the config file settings.
    """

    settings: Settings
    raw_config: Dict[str, Any]

    def __init__(
        self,
        config_file: Path = Path("/etc/pacstall/config.toml"),
    ) -> None:
        """
        Initialize the config.

        Parameters
        ----------
        config_file
            The config file to parse.
        """

        config_file.touch(exist_ok=True)

        with config_file.open(mode="rb") as file:
            raw_config = load(file)

        self.raw_config = raw_config

        self.settings = Settings(raw_config)
