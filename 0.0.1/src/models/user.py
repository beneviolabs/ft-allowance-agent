from typing import Optional
from sqlalchemy import String, Numeric, func, Index
from sqlalchemy.dialects.postgresql import UUID
from sqlalchemy.orm import Mapped, mapped_column
import uuid

from src.models import Base


class User(Base):
    """
    User model representing a customer of our service.
    """

    __tablename__ = "users"

    id: Mapped[uuid.UUID] = mapped_column(
        UUID(as_uuid=True),
        primary_key=True,
        default=func.gen_random_uuid(),
    )
    near_account_id: Mapped[str] = mapped_column(String, nullable=False, unique=True)
    allowance_goal: Mapped[float] = mapped_column(Numeric(20, 8), nullable=True)
    growth_goal: Mapped[float] = mapped_column(Numeric(20, 8), nullable=True)

    def __repr__(self) -> str:
        return f"<User(id={self.id}, near_account_id='{self.near_account_id}', allowance_goal={self.allowance_goal}, growth_goal={self.growth_goal})>"
