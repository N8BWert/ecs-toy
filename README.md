# ECS Test Program

## Description

I thought it might be interesting to make a toy ECS system, so this is that system.  The program basically simulates particles randomly moving around a grid.  The visual is relatively neat to watch, but the main goal of this project had nothing to do with the particles that are moving and, instead, had to do with making an ECS system that was capable to utilizing a high degree of parallelism.

When testing on an i7 mac I was between 20-35 milliseconds between frame renders (~29-50 frames per second) with 1,000,000 entities using 7 components and 8 systems on 5 workers.  I'd imagine its probably possible to get significantly better frames by combining the acceleration, velocity, and position systems (which likely block each other), but as a toy example I think this is performant enough.

## Interesting Behavior

The particles seem to swarm the center of the screen in the beginning and then rush to the edges with a few particles making journeys through the middle of the area.
