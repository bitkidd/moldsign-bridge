"""Independently verify the full fixture's timestamp using macOS XML/OpenSSL tools."""
import base64
from pathlib import Path
import subprocess
import sys
from xml.dom import minidom

root = Path(sys.argv[1])
document = minidom.parse(str(root / 'signature.xml'))
ds = 'http://www.w3.org/2000/09/xmldsig#'
xades = 'http://uri.etsi.org/01903/v1.3.2#'
value = document.getElementsByTagNameNS(ds, 'SignatureValue')[0].cloneNode(True)
value.setAttribute('xmlns:ds', ds)
canonical = subprocess.run(['/usr/bin/xmllint', '--c14n', '-'], input=value.toxml().encode(), capture_output=True, check=True).stdout
stamp = document.getElementsByTagNameNS(xades, 'EncapsulatedTimeStamp')[0]
assert stamp.getAttribute('Encoding') == 'http://uri.etsi.org/01903/v1.2.2#DER'
encoded = ''.join(node.data for node in stamp.childNodes if node.nodeType == node.TEXT_NODE)
(root / 'extracted-timestamp.der').write_bytes(base64.b64decode(encoded))
(root / 'independent-timestamp-input.xml').write_bytes(canonical)
subprocess.run(['/usr/bin/openssl', 'ts', '-verify', '-token_in', '-in', str(root / 'extracted-timestamp.der'), '-data', str(root / 'independent-timestamp-input.xml'), '-CAfile', str(root / 'ca.pem')], check=True)
print('Independent timestamp signature, imprint, and test trust: valid')
