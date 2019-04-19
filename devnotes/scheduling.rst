scheduler:
 * decides what packages need processing based on UDD output
 * adds packages to the queue
 * applies policy.conf

(also: have commands/API for manually adding things to the queue.

runner:
 * processes the queue, delegating work to workers

worker(s):
 * processes a single package
