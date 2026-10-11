while (!iterator.isDone()) {
			Piki* c = iterator.mContainer->get(iterator.mIndex);

			if (c->isAlive() && c->isPikmin() && !c->isStickToMouth() && c->mSticker != this && c->mFloorTriangle) {
				f32 ang = getAngDist(c);
				if (FABS(ang) <= maxAngle) {
					Vector3f pos = c->getPosition();
					if (sqrDistanceXZ(mPosition, pos) < maxDist) {
						return c;
					}
				}
			}
			iterator.next();
		}