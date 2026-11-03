"""Write ephemeral signing files without logging their contents."""
import base64
import os
from pathlib import Path

required = ['APPLE_CERTIFICATE', 'APPLE_CERTIFICATE_PASSWORD', 'APPLE_SIGNING_IDENTITY',
            'APPLE_API_ISSUER', 'APPLE_API_KEY', 'APPLE_API_PRIVATE_KEY', 'TAURI_SIGNING_PRIVATE_KEY']
missing = [name for name in required if not os.environ.get(name)]
if missing:
    raise SystemExit('Configure release environment secrets: ' + ', '.join(missing))
folder = Path(os.environ['RUNNER_TEMP'])
(folder / 'certificate.p12').write_bytes(base64.b64decode(os.environ['APPLE_CERTIFICATE'], validate=True))
(folder / 'AuthKey.p8').write_text(os.environ['APPLE_API_PRIVATE_KEY'])
for name in ['certificate.p12', 'AuthKey.p8']:
    (folder / name).chmod(0o600)
