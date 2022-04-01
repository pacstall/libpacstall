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

"""Test suit for the cache module."""

from datetime import datetime
from typing import Generator

import pytest
from sqlalchemy.exc import IntegrityError
from sqlmodel import Session, SQLModel, create_engine, select

from libpacstall.cache import (
    APTDependency,
    InstallStatus,
    Pacscript,
    PacscriptDependency,
    Source,
)


@pytest.fixture
def session() -> Generator[Session, None, None]:
    """
    Create a session for the tests.

    Yields
    ------
    Session
        The database session.
    """

    engine = create_engine("sqlite://", echo=False)
    SQLModel.metadata.create_all(engine)
    yield Session(engine)


class TestWrites:
    """Test that writing to the database works."""

    def test_full_write(self, session: Session) -> None:
        """
        Test that we can write a full records to the database.

        Parameters
        ----------
        session
            The session database to use (Fixture)
        """

        with session:
            session.add(
                Pacscript(
                    name="foo",
                    version="1.0",
                    url="https://foo.bar",
                    homepage="https://foo.bar",
                    description="baz",
                    source=Source(
                        url="https://bar.baz", last_updated=datetime.now(), preference=2
                    ),
                    installed_size=420,
                    download_size=69,
                    date=datetime.now(),
                    apt_dependencies=[
                        APTDependency(
                            name="foo",
                        )
                    ],
                    apt_optional_dependencies={"bar": "baz"},
                    pacscript_dependencies=[
                        PacscriptDependency(
                            name="foo",
                        )
                    ],
                    pacscript_optional_dependencies={"bit": "bat"},
                    install_status=InstallStatus.DIRECT,
                    repology={"project": "foo", "visiblename": "bar"},
                    maintainer="baz <foo@baz.com>",
                )
            )

            session.commit()

    def test_partial_write(self, session: Session) -> None:
        """
        Test that partial writes to the database are possible.

        Parameters
        ----------
        session
            The session to use (Fixture)
        """

        with session:
            session.add(
                Pacscript(
                    name="foo",
                    version="1.0",
                    url="https://foo.bar",
                    description="baz",
                    download_size=69,
                    date=datetime.now(),
                )
            )
            session.commit()

    def test_session_integrity(self, session: Session) -> None:
        """
        Test that we can't add two pacscripts of the same name and
        install_status.

        Parameters
        ----------
        session
            The session to use (Fixture)
        """

        with session:
            session.add(
                Pacscript(
                    name="foo",
                    version="1.0",
                    url="https://foo.bar",
                    homepage="https://foo.bar",
                    description="baz",
                    source=Source(
                        url="https://bar.baz", last_updated=datetime.now(), preference=2
                    ),
                    installed_size=420,
                    download_size=69,
                    date=datetime.now(),
                    apt_dependencies=[
                        APTDependency(
                            name="foo",
                        )
                    ],
                    apt_optional_dependencies={"bar": "baz"},
                    install_status=InstallStatus.DIRECT,
                    repology={"project": "foo", "visiblename": "bar"},
                    maintainer="baz <foo@baz.com>",
                )
            )

            session.add(
                Pacscript(
                    name="foo",
                    version="1.0",
                    url="https://foo.bar",
                    homepage="https://foo.bar",
                    description="baz",
                    source=Source(
                        url="https://bar.baz", last_updated=datetime.now(), preference=2
                    ),
                    installed_size=420,
                    download_size=69,
                    date=datetime.now(),
                    apt_dependencies=[
                        APTDependency(
                            name="foo",
                        )
                    ],
                    apt_optional_dependencies={"bar": "baz"},
                    install_status=InstallStatus.DIRECT,
                    repology={"project": "foo", "visiblename": "bar"},
                    maintainer="baz <foo@baz.com>",
                )
            )
            with pytest.raises(IntegrityError):
                session.commit()

    def test_installed_and_source_entries(self, session: Session) -> None:
        """
        Test that we can write installed and source pacscript entries into the
        database.

        Parameters
        ----------
        session
            The session to use (Fixture)
        """

        with session:
            session.add(
                Pacscript(
                    name="foo",
                    version="1.0",
                    url="https://foo.bar",
                    description="baz",
                    install_status=InstallStatus.DIRECT,
                    download_size=69,
                    date=datetime.now(),
                )
            )

            session.add(
                Pacscript(
                    name="foo",
                    version="1.0",
                    url="https://foo.bar",
                    description="baz",
                    download_size=69,
                    date=datetime.now(),
                )
            )

            session.commit()


class TestReads:
    """Test that reading from the database works."""

    def test_read_all(self, session: Session) -> None:
        """
        Test that we can read all pacscripts from the database.

        Parameters
        ----------
        session
            The session to use (Fixture)
        """

        current_date_time = datetime.now()

        installed_pacsript = Pacscript(
            name="foo",
            version="1.0",
            url="https://foo.bar",
            description="baz",
            install_status=InstallStatus.DIRECT,
            download_size=69,
            date=current_date_time,
        )

        source_pacscript = Pacscript(
            name="foo",
            version="2.0",
            url="https://foo.bar",
            description="baz",
            download_size=69,
            date=current_date_time,
        )

        with session:
            session.add(installed_pacsript)
            session.add(source_pacscript)
            session.commit()

        installed_pacscript_list = session.exec(
            select(Pacscript).where(Pacscript.install_status == InstallStatus.DIRECT)
        ).all()

        source_pacscript_list = session.exec(
            select(Pacscript).where(
                Pacscript.install_status == InstallStatus.NOT_INSTALLED
            )
        ).all()

        assert len(session.exec(select(Pacscript)).all()) == 2

        assert len(installed_pacscript_list) == 1
        assert len(source_pacscript_list) == 1

        assert installed_pacscript_list[0].version == "1.0"
        assert installed_pacscript_list[0].install_status == InstallStatus.DIRECT

        assert source_pacscript_list[0].version == "2.0"
        assert source_pacscript_list[0].install_status == InstallStatus.NOT_INSTALLED

    def test_read_by_name(self, session: Session) -> None:
        """
        Test that we can read a pacscript by name from the database.

        Parameters
        ----------
        session
            The session to use (Fixture)
        """

        with session:
            session.add(
                Pacscript(
                    name="foo",
                    version="1.0",
                    url="https://foo.bar",
                    description="baz",
                    download_size=69,
                    date=datetime.now(),
                )
            )

            session.add(
                Pacscript(
                    name="bar",
                    version="1.0",
                    url="https://foo.bar",
                    description="baz",
                    download_size=69,
                    date=datetime.now(),
                )
            )

            session.commit()

        assert len(session.query(Pacscript).filter(Pacscript.name == "bar").all()) == 1
        assert len(session.query(Pacscript).filter(Pacscript.name == "foo").all()) == 1
