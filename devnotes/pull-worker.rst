At the moment, all workers are started by the runner.

This works well for a small number of workers that are initiated by the runner.

It makes e.g. running on Jenkins harder. For Jenkins, we would ideally have
a pull worker:

1) A worker starts up independently, with some indication of where the runner lives.
2) It connects to the runner and retrieves a description of a job to do.
 + The runner now marks this queue entry as "in progress" by the particular
   worker.
3) It executes the job, and streams status to the runner.
4) When it is done, it uploads the build artefacts to the archiver.

Challenges:
 + The worker needs to authenticate itself to the runner.
 + If the worker dies, the runner at some point needs to resuscitate the job.
 + The worker needs to authenticate itself to the archiver

For authentication, the most obvious option to me appears to be SSL certs.

Roadmap
=======

1) Track who built a particular run - need to add SQL field and populate that
1) Make the /publish API externally accessible, but require SSL authentication
1) Make the runner "reserve" slots for certain workers in the queue
1) Interaction with the runner
 a) Add an API to the runner to allow polling for the next
    job to process.
 b) Provide a websocket API to allow streaming the logs to the runner (optional)
 c) Add an API to allow reporting run result to the runner
