import logging
import sys


def configure_logging():
    """Configure root logger with console handler"""
    # Create handlers
    c_handler = logging.StreamHandler(sys.stdout)
    c_handler.setLevel(logging.DEBUG)

    # Create formatters and add it to handlers
    c_format = logging.Formatter(
        '%(asctime)s - %(name)s - %(levelname)s - %(message)s')
    c_handler.setFormatter(c_format)

    # Get root logger
    root_logger = logging.getLogger()
    root_logger.setLevel(logging.DEBUG)

    # Remove existing handlers to prevent duplicate logs
    root_logger.handlers = []

    # Add handlers to the logger
    root_logger.addHandler(c_handler)
