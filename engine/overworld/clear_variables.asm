ClearVariablesOnEnterMap::
	ld a, SCREEN_HEIGHT_PX
	ldh [hWY], a
	ldh [rWY], a
	xor a
	ldh [hAutoBGTransferEnabled], a
	ld [wStepCounter], a
	ld [wLoneAttackNo], a
	ldh [hJoyPressed], a
	ldh [hJoyReleased], a
	ldh [hJoyHeld], a
	ld [wActionResultOrTookBattleTurn], a
	ld [wUnusedMapVariable], a
	ld hl, wCardKeyDoorY
	ld [hli], a
	ld [hl], a
	ld hl, wWhichTrade
	ld bc, wStandingOnWarpPadOrHole - wWhichTrade
	call FillMemory
; fix: clear stale ledge jump / simulated-input state (Walking Through Walls).
; If a forced warp (Safari Zone timer expiry) interrupts a ledge jump,
; BIT_LEDGE_OR_FISHING persists across the map transition. CollisionCheckOnLand
; checks this bit first and unconditionally skips all tile collision, letting the
; player walk through walls on the destination map. Similarly, a non-zero
; wSimulatedJoypadStatesIndex bypasses collision for the remaining phantom steps.
; https://bulbapedia.bulbagarden.net/wiki/Walking_through_walls
; https://glitchcity.wiki/wiki/Walk_through_walls_trick_(ledge_method)
	ld hl, wMovementFlags
	res BIT_LEDGE_OR_FISHING, [hl]
	ld [wSimulatedJoypadStatesIndex], a ; a = 0 from FillMemory
; Clear stale trainer encounter state (prevents Trainer Fly / Mew glitch).
; BIT_SEEN_BY_TRAINER persists across map transitions if the player warps
; away (Fly/Teleport/Dig) after a trainer spots them but before battle starts.
; https://bulbapedia.bulbagarden.net/wiki/Mew_glitch
	ld hl, wMiscFlags
	res BIT_SEEN_BY_TRAINER, [hl]
	ret
