import logging

log = logging.getLogger(__name__)


class LoggerAdapter:
    def __init__(self, env=None):
        self.env = env

    def debug(self, message: str):
        if self.env:
            self.env.add_system_log(message, logging.DEBUG)
        else:
            log.debug(message)

    def info(self, message: str):
        if self.env:
            self.env.add_system_log(message, logging.INFO)
        else:
            log.info(message)

    def error(self, message: str):
        if self.env:
            self.env.add_system_log(message, logging.ERROR)
        else:
            log.error(message)
