from sqlalchemy import create_engine
from sqlalchemy.orm import Session, sessionmaker
from sqlalchemy.engine import Engine


def get_session_factory(db_url: str) -> sessionmaker:
    """
    Initializes the database engine and returns a session factory.
    """
    engine = create_engine(db_url, echo=True)
    return sessionmaker(
        bind=engine,
    )
