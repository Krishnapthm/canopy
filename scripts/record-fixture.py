#!/usr/bin/env python3
"""Write scrubbed JSONL from stdin: probe | python3 scripts/record-fixture.py FILE."""
import json
import re
import sys


def scrub(value):
    if isinstance(value, dict):
        clean = {}
        for key, item in value.items():
            if key == 'memory_paths':
                clean[key] = {}
            elif key in {'plugins', 'mcp_servers', 'slash_commands', 'skills', 'agents',
                         'commands', 'tools', 'terminal_slash_commands', 'additional_directories'} and isinstance(item, list):
                clean[key] = []
            elif key in {'email', 'organization', 'orgId', 'orgName', 'hostname', 'username', 'userName'}:
                clean[key] = '[redacted]'
            else:
                clean[key] = scrub(item)
        return clean
    if isinstance(value, list):
        return [scrub(v) for v in value]
    if isinstance(value, str):
        try:
            embedded = json.loads(value)
        except (ValueError, TypeError):
            pass
        else:
            if isinstance(embedded, (dict, list)):
                return json.dumps(scrub(embedded), separators=(',', ':'))
        value = re.sub(r'/(?:Users|home)/[^/\s"\\]+', '/home/fixture', value)
        value = re.sub(r'/(?:private/)?(?:tmp|var/folders)/[^\s"\\]+', '/tmp/fixture', value)
        return re.sub(r'\b(?:Users|home)-[^-\s/"\\]+', 'home-fixture', value)
    return value


def self_test():
    sample = {'cwd': '/Users/recorder/project', 'plugins': ['private'],
              'mcp_servers': [{'name': 'private'}], 'tools': 12, 'capabilities': {'tools': {}}, 'account': {'email': 'private'},
              'body': '{"path":"/home/recorder/project"}'}
    clean = scrub(sample)
    assert clean['tools'] == 12 and clean['capabilities'] == {'tools': {}}
    assert clean['cwd'] == '/home/fixture/project'
    assert clean['plugins'] == clean['mcp_servers'] == []
    assert clean['account'] == {'email': '[redacted]'}
    assert json.loads(clean['body'])['path'] == '/home/fixture/project'
    assert scrub('Users-recorder-projects') == 'home-fixture-projects'
    assert scrub(clean) == clean


if __name__ == '__main__' and sys.argv[1:] == ['--test']:
    self_test()
    sys.exit(0)

if __name__ == '__main__':
    with open(sys.argv[1], 'w') as out:
        for line in sys.stdin:
            out.write(json.dumps(scrub(json.loads(line)), separators=(',', ':')) + '\n')
