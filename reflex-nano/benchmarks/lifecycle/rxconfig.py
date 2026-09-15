import reflex as rx
from reflex_base.constants import StateManagerMode

config = rx.Config(
    app_name='lifecycle_app', state_manager_mode=StateManagerMode.MEMORY,
    api_url='http://127.0.0.1:3356', deploy_url='http://127.0.0.1:3355',
    transport='websocket', plugins=[], disable_plugins=[rx.plugins.SitemapPlugin],
    show_built_with_reflex=False, telemetry_enabled=False,
)
