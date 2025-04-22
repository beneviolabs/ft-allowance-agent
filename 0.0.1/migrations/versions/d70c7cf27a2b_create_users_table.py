"""create users table

Revision ID: d70c7cf27a2b
Revises:
Create Date: 2025-04-22 15:04:00.022372

"""

from typing import Sequence, Union

from alembic import op
import sqlalchemy as sa
from sqlalchemy.dialects import postgresql


# revision identifiers, used by Alembic.
revision: str = "d70c7cf27a2b"
down_revision: Union[str, None] = None
branch_labels: Union[str, Sequence[str], None] = None
depends_on: Union[str, Sequence[str], None] = None


def upgrade() -> None:
    """Upgrade schema."""
    op.create_table(
        "users",
        sa.Column(
            "id",
            postgresql.UUID(as_uuid=True),
            primary_key=True,
            server_default=sa.text("gen_random_uuid()"),
        ),
        sa.Column("near_account_id", sa.String, nullable=False),
        sa.Column("allowance_goal", sa.Numeric(20, 8), nullable=True),
        sa.Column("growth_goal", sa.Numeric(20, 8), nullable=True),
    )

    # Create unique index on near_account_id
    op.create_index(
        "ix_users_near_account_id", "users", ["near_account_id"], unique=True
    )


def downgrade() -> None:
    """Downgrade schema."""
    # Drop the unique index first
    op.drop_index("ix_users_near_account_id", table_name="users")
    # Drop the users table
    op.drop_table("users")
