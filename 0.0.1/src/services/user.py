from src.models.user import User
from sqlalchemy.orm import Session

from src.common import DivvyGoalType
from decimal import Decimal


def create_user(db: Session, user: User) -> User:
    """
    Create a new user in the database.
    """
    user = User(
        near_account_id=user.near_account_id,
    )
    db.add(user)
    db.commit()
    return user


def get_user_by_near(db: Session, near_id: str) -> User | None:
    """
    Get a user by near_account_id.
    """
    return db.query(User).filter(User.near_account_id == near_id).first()


def update_user_goals(
    db: Session,
    user: User,
    goal_type: DivvyGoalType,
    value: Decimal,
) -> User:
    """
    Update the user's allowance and growth goals.
    """
    if goal_type == "allowance":
        user.allowance_goal = value
    if goal_type == "growth":
        user.growth_goal = value
    db.commit()
    return user
