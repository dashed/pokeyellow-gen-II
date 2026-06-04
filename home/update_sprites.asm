UpdateSprites::
	ld a, [wUpdateSpritesEnabled]
	dec a
	ret nz
	ldh a, [hLoadedROMBank]
	push af
	ld a, BANK(_UpdateSprites)
	call BankswitchCommon
	dec a ; A was 0 after the dec/ret nz check; dec wraps to $FF
	ld [wUpdateSpritesEnabled], a
	ldh [hOAMUpdateLocked], a ; lock OAM DMA while building the buffer
	call _UpdateSprites
	xor a
	ldh [hOAMUpdateLocked], a ; unlock — VBlank may now transfer OAM
	ld a, $1
	ld [wUpdateSpritesEnabled], a
	pop af
	jp BankswitchCommon
