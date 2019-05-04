::
   E.g.: python-potr

   Traceback (most recent call last):
     File "/usr/lib/python3.7/runpy.py", line 193, in _run_module_as_main
       "__main__", mod_spec)
     File "/usr/lib/python3.7/runpy.py", line 85, in _run_code
       exec(code, run_globals)
     File "/home/janitor/debian-janitor/janitor/runner.py", line 622, in <module>
       sys.exit(main(sys.argv))
     File "/home/janitor/debian-janitor/janitor/runner.py", line 617, in main
       loop.create_task(run_web_server(args.listen_address, args.port)),
     File "/usr/lib/python3.7/asyncio/base_events.py", line 584, in run_until_complete
       return future.result()
     File "/home/janitor/debian-janitor/janitor/runner.py", line 529, in process_queue
       log_dir=log_dir)
     File "/home/janitor/debian-janitor/janitor/runner.py", line 363, in process_one
       vcs_url, possible_transports=possible_transports)
     File "/home/janitor/debian-janitor/silver-platter/silver_platter/utils.py", line 149, in open_branch
       return Branch.open(url, possible_transports=possible_transports)
     File "/usr/lib/python3/dist-packages/breezy/branch.py", line 178, in open
       _unsupported=_unsupported)
     File "/usr/lib/python3/dist-packages/breezy/controldir.py", line 707, in open
       _unsupported=_unsupported)
     File "/usr/lib/python3/dist-packages/breezy/controldir.py", line 737, in open_from_transport
       find_format, transport, redirected)
     File "/usr/lib/python3/dist-packages/breezy/transport/__init__.py", line 1613, in do_catching_redirections
       return action(transport)
     File "/usr/lib/python3/dist-packages/breezy/controldir.py", line 725, in find_format
       probers=probers)
     File "/usr/lib/python3/dist-packages/breezy/controldir.py", line 1162, in find_format
       return prober.probe_transport(transport)
     File "/usr/lib/python3/dist-packages/breezy/git/__init__.py", line 200, in probe_transport
       return self.probe_http_transport(transport)
     File "/usr/lib/python3/dist-packages/breezy/git/__init__.py", line 174, in probe_http_transport
       resp = transport._perform(req)
     File "/usr/lib/python3/dist-packages/breezy/transport/http/__init__.py", line 116, in _perform
       response = self._opener.open(request)
     File "/usr/lib/python3.7/urllib/request.py", line 531, in open
       response = meth(req, response)
     File "/usr/lib/python3/dist-packages/breezy/transport/http/_urllib2_wrappers.py", line 1846, in http_response
       code, msg, hdrs)
     File "/usr/lib/python3.7/urllib/request.py", line 563, in error
       result = self._call_chain(*args)
     File "/usr/lib/python3.7/urllib/request.py", line 503, in _call_chain
       result = func(*args)
     File "/usr/lib/python3/dist-packages/breezy/transport/http/_urllib2_wrappers.py", line 1757, in http_error_401
       return self.auth_required(req, headers)
     File "/usr/lib/python3/dist-packages/breezy/transport/http/_urllib2_wrappers.py", line 1334, in auth_required
       raise KeyError('%s not found' % self.auth_required_header)
   KeyError: 'www-authenticate not found'


.. include:: 401-without-www-authenticate-list.rst
