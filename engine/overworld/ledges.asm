HandleLedges::
	ld a, [wMovementFlags]
	bit BIT_LEDGE_OR_FISHING, a
	ret nz
	ld a, [wCurMapTileset]
	and a ; OVERWORLD
	ret nz
	predef GetTileAndCoordsInFrontOfPlayer
	ld a, [wSpritePlayerStateData1FacingDirection]
	ld b, a
	lda_coord 8, 9
	ld c, a
	ld a, [wTileInFrontOfPlayer]
	ld d, a
	ld hl, LedgeTiles
.loop
	ld a, [hli]
	cp $ff
	ret z
	cp b
	jr nz, .nextLedgeTile1
	ld a, [hli]
	cp c
	jr nz, .nextLedgeTile2
	ld a, [hli]
	cp d
	jr nz, .nextLedgeTile3
	ld a, [hl]
	ld e, a
	jr .foundMatch
.nextLedgeTile1
	inc hl
.nextLedgeTile2
	inc hl
.nextLedgeTile3
	inc hl
	jr .loop
.foundMatch
	ldh a, [hJoyHeld]
	and e
	ret z
; Check if an NPC sprite occupies the landing tile (2 tiles ahead).
; Without this check, the player can jump a ledge and land on top of an NPC.
; https://glitchcity.wiki/wiki/NPC_collision_bypassing_glitch
; https://bulbapedia.bulbagarden.net/wiki/List_of_overworld_glitches_in_Generation_I
	push de
	xor a
	ldh [hSpriteIndex], a
	ld d, $20 ; 2-tile range (ledge jump landing distance)
	call IsSpriteInFrontOfPlayer2
	ldh a, [hSpriteIndex]
	and a ; NPC at landing position?
	pop de
	ret nz ; cancel jump if sprite is in the way
	ld a, PAD_BUTTONS | PAD_CTRL_PAD
	ld [wJoyIgnore], a
	ld hl, wMovementFlags
	set BIT_LEDGE_OR_FISHING, [hl]
	call StartSimulatingJoypadStates
	ld a, e
	ld [wSimulatedJoypadStatesEnd], a
	ld [wSimulatedJoypadStatesEnd + 1], a
	ld a, $2
	ld [wSimulatedJoypadStatesIndex], a
	call LoadHoppingShadowOAM
	ld a, SFX_LEDGE
	call PlaySound
	ret

INCLUDE "data/tilesets/ledge_tiles.asm"

LoadHoppingShadowOAM:
	ld hl, vChars1 tile $7f
	ld de, LedgeHoppingShadow
	lb bc, BANK(LedgeHoppingShadow), (LedgeHoppingShadowEnd - LedgeHoppingShadow) / TILE_1BPP_SIZE
	call CopyVideoDataDouble
	ld hl, LedgeHoppingShadowOAM
	ld de, wShadowOAMSprite36
	ld bc, LedgeHoppingShadowOAMEnd - LedgeHoppingShadowOAM
	call CopyData
	ld a, $a0
	ld [wShadowOAMSprite38YCoord], a
	ld [wShadowOAMSprite39YCoord], a
	ret

LedgeHoppingShadow:
	INCBIN "gfx/overworld/shadow.1bpp"
LedgeHoppingShadowEnd:

LedgeHoppingShadowOAM:
	dbsprite  9, 11,  0,  0, $ff, 0
	dbsprite 10, 11,  0,  0, $ff, OAM_XFLIP
LedgeHoppingShadowOAMEnd:
