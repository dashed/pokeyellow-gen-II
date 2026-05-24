DisplayElevatorFloorMenu:
	ld hl, wStatusFlags5
	ld a, [hl]
	push af
	set BIT_NO_TEXT_DELAY, [hl]
	ld hl, WhichFloorText
	call PrintText
	pop af
	ld [wStatusFlags5], a
	ld hl, wItemList
	ld a, l
	ld [wListPointer], a
	ld a, h
	ld [wListPointer + 1], a
	ld a, [wListScrollOffset]
	push af
	xor a
	ld [wCurrentMenuItem], a
	ld [wListScrollOffset], a
	ld [wPrintItemPrices], a
	ld a, SPECIALLISTMENU
	ld [wListMenuID], a
	call DisplayListMenuID
	pop bc
	ld a, b
	ld [wListScrollOffset], a
	ret c
	ld hl, wElevatorWarpMaps
	ld a, [wWhichPokemon]
	add a
	ld d, 0
	ld e, a
	add hl, de
	ld a, [hli]
	ld b, a
	ld a, [hl]
	ld c, a
; fix: skip elevator if the selected floor is the one we came from.
; Without this, the shake animation plays and the player "warps" to the
; same map they entered from, wasting time.
; https://bulbapedia.bulbagarden.net/wiki/List_of_overworld_glitches_in_Generation_I
	ld a, [wWarpedFromWhichMap]
	cp c
	ret z ; same floor — do nothing
	ld hl, wCurrentMapScriptFlags
	set BIT_CUR_MAP_USED_ELEVATOR, [hl]
	ld hl, wWarpEntries
	call .UpdateWarp

.UpdateWarp
	inc hl
	inc hl
	ld a, b
	ld [hli], a ; destination warp ID
	ld a, c
	ld [hli], a ; destination map ID
	ret

WhichFloorText:
	text_far _WhichFloorText
	text_end
