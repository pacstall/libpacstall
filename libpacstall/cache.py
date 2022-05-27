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

"""Caching system for pacscripts."""

from datetime import datetime
from enum import Enum, auto
from typing import Dict, List, Optional

from sqlalchemy.types import JSON
from sqlalchemy.types import Enum as SQLAlchemyEnum
from sqlmodel import Column, Field, Relationship, SQLModel
from sqlmodel.sql.expression import Select, SelectOfScalar

# HACK: https://github.com/tiangolo/sqlmodel/issues/189#issuecomment-1065790432
SelectOfScalar.inherit_cache = True  # type: ignore
Select.inherit_cache = True  # type: ignore


class InstallStatus(Enum):
    """
    Status of an installed package.

    Attributes
    ----------
    NOT_INSTALLED
        The dependency wasn't installed, indicating a source pacscript.
    DIRECT
        The dependency was directly installed by the user.
    INDIRECT
        The dependency was installed by another dependency.
    """

    NOT_INSTALLED = auto()
    DIRECT = auto()
    INDIRECT = auto()


class APTDependencyPacscriptLink(SQLModel, table=True):
    """
    Link between an APT dependency and a pacscript.

    Attributes
    ----------
    dependency_name
        The name of the dependency.
    pacscript_name
        The name of the pacscript.
    """

    dependency_name: Optional[int] = Field(
        default=None, foreign_key="aptdependency.name", primary_key=True
    )
    pacscript_name: Optional[str] = Field(
        default=None, foreign_key="pacscript.name", primary_key=True
    )


class APTDependency(SQLModel, table=True):
    """
    SQLModel of an APT dependency for a pacscript.

    Attributes
    ----------
    name
        Name of the dependency.
    dependents
        List of pacscripts that depend on this dependency.
    """

    name: str = Field(primary_key=True)
    dependents: List["Pacscript"] = Relationship(
        back_populates="apt_dependencies", link_model=APTDependencyPacscriptLink
    )


class PacscriptDependencyLink(SQLModel, table=True):
    """
    Link between a pacscript dependency and a pacscript.

    Attributes
    ----------
    pacscript_name
        Name of the pacscript.
    dependency_name
        Name of the dependency.
    """

    pacscript_name: Optional[str] = Field(
        default=None, foreign_key="pacscript.name", primary_key=True
    )
    dependency_name: Optional[str] = Field(
        default=None, foreign_key="pacscriptdependency.name", primary_key=True
    )


class PacscriptDependency(SQLModel, table=True):
    """
    SQLModel of a pacscript dependency of a pacscript.

    Attributes
    ----------
    name
        Name of the dependency.
    dependents
        List of pacscripts that depend on this dependency.
    """

    name: str = Field(primary_key=True)
    dependents: List["Pacscript"] = Relationship(
        back_populates="pacscript_dependencies", link_model=PacscriptDependencyLink
    )


class Source(SQLModel, table=True):
    """
    SQLModel of a source.

    Attributes
    ----------
    url
        URL of the source.
    last_updated
        Last time the source was updated.
    preference
        Preference of the source.
    pacscripts
        List of pacscripts that are from this source.
    """

    url: str = Field(index=True, primary_key=True)
    last_updated: datetime
    preference: int
    pacscripts: List["Pacscript"] = Relationship(back_populates="source")


class Pacscript(SQLModel, table=True):
    """
    SQLModel to access and write to the Pacscript database.

    There are two types of pacscripts stored in this table, installed
    pacscripts, and source pacscripts. If the `Install_status` column is NULL
    then the pacscript is a source pacscript. Otherwise it's an installed one.

    Attributes
    ----------
    name
        The name of the pacscript.
    version
        The version of the pacscript.
    url
        The URL of the pacscript.
    homepage
        The homepage of the pacscript.
    description
        The description of the pacscript.
    source_url_id
        The URL of the source of the pacscript. (Foreign key)
    source
        The source associated with the pacscript.
    installed_size
        The installed size of the pacscript's package in bytes.
    download_size
        The downloaded size of the pacscript's package in bytes.
    date
        The date the pacscript was last updated.
    install_status
        The installed status of the pacscript.
    apt_dependencies
        The list of apt dependencies of the pacscript.
    apt_optional_dependencies
        The list of apt optional dependencies of the pacscript.
    pacscript_dependencies
        The list of pacscript dependencies of the pacscript.
    pacscript_optional_dependencies
        The list of pacscript optional dependencies of the pacscript.
    repology
        The repology filters for the pacscript.
    maintainer
        The maintainer of the pacscript.
    """

    # Primary keys
    name: str = Field(index=True, primary_key=True)
    install_status: Optional[InstallStatus] = Field(
        default=InstallStatus.NOT_INSTALLED,
        sa_column=Column(SQLAlchemyEnum(InstallStatus), primary_key=True),
    )

    # Metadata defined in the pacscript
    version: str
    url: str
    homepage: Optional[str] = None
    description: str
    repology: Optional[Dict[str, str]] = Field(default=None, sa_column=Column(JSON))
    maintainer: Optional[str] = None

    # Link to the pacscript source
    source_url_id: Optional[int] = Field(default=None, foreign_key="source.url")
    source: Source = Relationship(back_populates="pacscripts")

    # Metadata generated about the pacscript
    installed_size: Optional[int] = None
    download_size: int
    date: datetime

    # Dependencies
    apt_dependencies: Optional[List[APTDependency]] = Relationship(
        back_populates="dependents",
        link_model=APTDependencyPacscriptLink,
    )
    apt_optional_dependencies: Optional[Dict[str, str]] = Field(
        default=None, sa_column=Column(JSON)
    )
    pacscript_dependencies: Optional[List[PacscriptDependency]] = Relationship(
        back_populates="dependents",
        link_model=PacscriptDependencyLink,
    )
    pacscript_optional_dependencies: Optional[Dict[str, str]] = Field(
        default=None, sa_column=Column(JSON)
    )
