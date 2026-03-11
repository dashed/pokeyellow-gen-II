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
; Clear stale trainer encounter state (prevents Trainer Fly / Mew glitch).
; BIT_SEEN_BY_TRAINER persists across map transitions if the player warps
; away (Fly/Teleport/Dig) after a trainer spots them but before battle starts.
; https://bulbapedia.bulbagarden.net/wiki/Mew_glitch
	ld hl, wMiscFlags
	res BIT_SEEN_BY_TRAINER, [hl]
	ret
