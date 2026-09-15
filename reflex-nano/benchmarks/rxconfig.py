import reflex as rx
from reflex_base.constants import StateManagerMode

config=rx.Config(app_name='reflex_app',state_manager_mode=StateManagerMode.MEMORY,
    transport='websocket',plugins=[],disable_plugins=[rx.plugins.SitemapPlugin],
    show_built_with_reflex=False,telemetry_enabled=False)
