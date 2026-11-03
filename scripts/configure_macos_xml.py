"""Link to the SDK's system libxml2 rather than a runner's Homebrew library."""
import os
import re
import subprocess
from pathlib import Path

sdk = Path(subprocess.check_output(['xcrun', '--show-sdk-path'], text=True).strip())
header = (sdk / 'usr/include/libxml2/libxml/xmlversion.h').read_text()
version = re.search(r'#define LIBXML_DOTTED_VERSION "([^"]+)"', header).group(1)
folder = Path(os.environ['RUNNER_TEMP']) / 'system-pkgconfig'
folder.mkdir(exist_ok=True)
(folder / 'libxml-2.0.pc').write_text(f'''Name: libXML
Description: macOS system XML library
Version: {version}
Libs: -L{sdk}/usr/lib -lxml2
Cflags: -I{sdk}/usr/include/libxml2
''')
with Path(os.environ['GITHUB_ENV']).open('a') as output:
    output.write(f'PKG_CONFIG_PATH={folder}\n')
