UpdateSprites::
	ld a, [wUpdateSpritesEnabled]
	dec a
	ret nz
	ldh a, [hLoadedROMBank]
	push af
	ld a, BANK(_UpdateSprites)
	call BankswitchCommon
	ld a, $ff
	ld [wUpdateSpritesEnabled], a
	ldh [hOAMUpdateLocked], a ; lock OAM DMA while building the buffer
	call _UpdateSprites
	xor a
	ldh [hOAMUpdateLocked], a ; unlock — VBlank may now transfer OAM
	ld a, $1
	ld [wUpdateSpritesEnabled], a
	pop af
	call BankswitchCommon
	ret
