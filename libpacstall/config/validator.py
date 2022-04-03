#     __    _ __    ____                  __        ____
#    / /   (_) /_  / __ \____ ___________/ /_____ _/ / /
#   / /   / / __ \/ /_/ / __ `/ ___/ ___/ __/ __ `/ / /
#  / /___/ / /_/ / ____/ /_/ / /__(__  ) /_/ /_/ / / /
# /_____/_/_.___/_/    \__,_/\___/____/\__/\__,_/_/_/
#
# Copyright (C) 2022-present
#
# This file is part of LibPacstall.
#
# LibPacstall is free software: you can redistribute it and/or modify it under the
# terms of the GNU General Public License as published by the Free Software
# Foundation, either version 3 of the License, or (at your option) any later
# version.
#
# LibPacstall is distributed in the hope that it will be useful, but WITHOUT ANY
# WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A
# PARTICULAR PURPOSE. See the GNU General Public License for more details.
#
# You should have received a copy of the GNU General Public License along with
# LibPacstall. If not, see <https://www.gnu.org/licenses/>.

"""Module for validating the config"""

from warnings import warn
import httpx


class MissingConfigValue(Exception):
    """Exception"""

    def __init__(self, cfg_key:str):
        self.message = f"{cfg_key}"
        super().__init__(self.message)

class InvalidSourceURL(Exception):
    """Exception raised for missing official repo"""

    def __init__(self, name:str):
        self.message = f"URL associated with the source {name} is unreachable"
        super().__init__(self.message)


def validate(raw_config: dict) -> None:
    """
    Validate raw config

    Parameters
    ----------
    raw_config
        Raw output of the config toml parser
    """
    config_types = {
        "settings" : dict,
        "repository" : dict
    }

    for cfg_key, cfg_value in raw_config.items():
        if cfg_value not in config_types:
            warn(f"{cfg_key} is not a valid key in the config file")
        elif cfg_value is not config_types[cfg_key]:
            raise TypeError(f"{cfg_key} should have type {config_types[cfg_key]}")

    for cfg_key in config_types:
        if cfg_key not in raw_config:
            raise MissingConfigValue(cfg_key)

    settings_types = {
        "jobs" : int,
        "editor" : str
    }

    settings = raw_config["repository"]

    for set_key, set_value in settings.items():
        if set_value not in settings_types:
            warn(f"{set_key} is not a valid key in the settings config")
        elif isinstance(set_value, settings_types[set_key]):
            raise TypeError(f"{cfg_key} should have type {settings_types[cfg_key]}")

    for set_key in setting_types:
        if cfg_key not in raw_config:
            raise MissingConfigValue(f"settings.{cfg_key}")

    repo_dict = raw_config["repository"]

    if "official" not in repo_dict:
        warn("Official pacstall source missing")

    for name, url in repo_dict.items():
        if not isinstance(url, str):
            raise TypeError(f"{name}'s url value should be of type str")
        #TODO: use url parser, get packagelist url instead
        if httpx.get(url).status_code not in [200, 301, 302]:
            raise InvalidSourceURL(name)
