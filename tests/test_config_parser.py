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

"""Tests for the config parser module."""

from os import cpu_count, environ
from pathlib import Path
from typing import Generator

import pytest

from libpacstall import config as libpacstall_config

boilerplate_toml = """
                   ## This is the boilerplate toml file's contents.

                   # Configure Pacstall settings here.
                   [settings]
                   jobs = 10 # Number of jobs to run in parallel.
                   editor = "nvim" # Editor to use for editing.
                   """


@pytest.fixture()
def config_file(tmp_path: Path) -> Generator[Path, None, None]:
    """
    Fixutre to create a config file.

    Parameters
    ----------
    tmp_path
        Path to the temporary directory. (fixture)

    Yields
    ------
    Path
        Path to the config file.
    """

    config_file = tmp_path / "config.toml"
    yield config_file
    config_file.unlink()


def test_raw_config(config_file: Path, tmp_path: Path) -> None:
    """
    Test that the raw config is parsed from the config file correctly.

    Parameters
    ----------
    config_file
        Path to the config file. (fixture)
    tmp_path
        Path to the temporary directory. (fixture)
    """

    config_file.write_text(boilerplate_toml)

    assert libpacstall_config.Config(config_file=config_file).raw_config == {
        "settings": {
            "jobs": 10,
            "editor": "nvim",
        }
    }


class TestSettings:
    def test_jobs(self, config_file: Path) -> None:
        """
        Test that the jobs setting is correctly parsed.

        Parameters
        ----------
        config_file
            Path to the config file. (fixture)
        """

        # Test boilerplate config.
        config_file.write_text(boilerplate_toml)
        assert libpacstall_config.Config(config_file=config_file).settings.jobs == 10

        # Test config with no jobs.
        jobless_toml = """
                       [settings]
                       editor = "nvim"
                       """

        config_file.write_text(jobless_toml)
        assert (
            libpacstall_config.Config(config_file=config_file).settings.jobs
            == cpu_count()
        )

    def test_editor(self, config_file: Path, monkeypatch: pytest.MonkeyPatch) -> None:
        """
        Test that the editor is set to the correct value.

        This test is a bit tricky because we need to mock the environ
        variable.

        Parameters
        ----------
        config_file
            Path to the config file. (fixture)
        monkeypatch
            The monkeypatch fixture.
        """

        # Test boilerplate config.
        config_file.write_text(boilerplate_toml)
        assert (
            libpacstall_config.Config(config_file=config_file).settings.editor == "nvim"
        )

        # Test config with no editor.
        editorless_toml = """
                          [settings]
                          jobs = 10
                          """

        config_file.write_text(editorless_toml)
        assert libpacstall_config.Config(
            config_file=config_file
        ).settings.editor == environ.get(
            "EDITOR", environ.get("VISUAL", "sensible-editor")
        )

        # Test config with no EDITOR environment variable.
        with monkeypatch.context() as monkey:
            monkey.delenv("EDITOR", raising=False)
            monkey.setenv("VISUAL", "TEST_EDITOR")

            config_file.write_text(editorless_toml)

            assert (
                libpacstall_config.Config(config_file=config_file).settings.editor
                == "TEST_EDITOR"
            )

        # Test config with no EDITOR and VISUAL environment variable.
        with monkeypatch.context() as monkey:
            monkey.delenv("EDITOR", raising=False)
            monkey.delenv("VISUAL", raising=False)

            assert (
                libpacstall_config.Config(config_file=config_file).settings.editor
                == "sensible-editor"
            )
