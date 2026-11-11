#!/usr/bin/python3
"""Synthetic extension wire fixture, not a recording of a live harness."""
import json
import os
from pathlib import Path
import sys

home = Path(os.environ['CANOPY_HOME'])
state_file = home / 'ext-state.json'
state = json.loads(state_file.read_text()) if state_file.exists() else {}

def save(key, value):
    state[key] = value
    state_file.write_text(json.dumps(state))

def plugins(scope):
    return [{'id': 'demo@local', 'enabled': state.get('claude:'+scope, True), 'scope': scope},
            {'id': 'declared@local', 'enabled': True, 'scope': scope, 'command': 'touch never-run'}]

if Path(sys.argv[0]).name == 'claude':
    args = sys.argv[1:]
    if args[:2] == ['plugin', 'list']:
        print(json.dumps(plugins('user') + plugins('project')))
    elif args[:2] in [['plugin', 'enable'], ['plugin', 'disable']]:
        assert args[2] == 'demo@local'
        save('claude:'+args[args.index('--scope')+1], args[1] == 'enable')
        print('{}')
    else:
        sys.exit(1)
else:
    for line in sys.stdin:
        msg = json.loads(line)
        if 'id' not in msg:
            continue
        method, params = msg['method'], msg.get('params', {})
        result = {}
        if method == 'plugin/installed':
            result = {'marketplaces': [{'name':'local','plugins':[{'id':'demo@local','name':'demo','installed':True,'enabled':state.get('codex:global', True)}]}]}
        elif method == 'skills/list':
            assert params['forceReload'] is True
            result = {'data':[{'skills':[{'name':'native','path':str(home / '.agents/skills/native/SKILL.md'),'enabled':state.get('skill', True),'scope':'user'}]}]}
        elif method == 'config/value/write':
            assert params['keyPath'] == 'plugins."demo@local".enabled'
            assert params['mergeStrategy'] == 'upsert'
            scope = 'project' if '/.codex/' in params['filePath'] and str(home / 'project') in params['filePath'] else 'global'
            save('codex:'+scope, params['value'])
        elif method == 'skills/config/write':
            assert params['path'] == str(home / '.agents/skills/native/SKILL.md')
            save('skill', params['enabled'])
        elif method not in ['initialize']:
            raise AssertionError(method)
        print(json.dumps({'id':msg['id'],'result':result}), flush=True)
