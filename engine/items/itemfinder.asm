HiddenItemNear:
	ld hl, HiddenItemCoords
	ld b, 0
.loop
	ld de, 3
	ld a, [wCurMap]
	call IsInRestOfArray
	ret nc ; return if current map has no hidden items
	push bc
	push hl
	ld hl, wObtainedHiddenItemsFlags
	ld c, b
	ld b, FLAG_TEST
	predef FlagActionPredef
	ld a, c
	pop hl
	pop bc
	inc b
	and a
	inc hl
	ld d, [hl]
	inc hl
	ld e, [hl]
	inc hl
	jr nz, .loop ; if the item has already been obtained
; check if the item is within 4-5 tiles (depending on the direction of item)
	ld a, [wYCoord]
	call Sub5ClampTo0
	cp d
; Fix: `jr nc` skips when A >= D, but equality means the item is at the
; detection boundary and should be found. Without this, items at Y=0 are
; missed when the player is at Y=0..5 (Sub5ClampTo0 clamps to 0).
	jr z, .checkYUpper
	jr nc, .loop
.checkYUpper
	ld a, [wYCoord]
	add 4
	cp d
	jr c, .loop
	ld a, [wXCoord]
	call Sub5ClampTo0
	cp e
; Fix: same off-by-one for X coordinate lower bound.
	jr z, .checkXUpper
	jr nc, .loop
.checkXUpper
	ld a, [wXCoord]
	add 5
	cp e
	jr c, .loop
	scf
	ret

Sub5ClampTo0:
; subtract 5 but clamp to 0
	sub 5
	cp $f0
	ret c
	xor a
	ret
