HealEffect_:
	ldh a, [hWhoseTurn]
	and a
	ld de, wBattleMonHP
	ld hl, wBattleMonMaxHP
	ld a, [wPlayerMoveNum]
	jr z, .healEffect
	ld de, wEnemyMonHP
	ld hl, wEnemyMonMaxHP
	ld a, [wEnemyMoveNum]
.healEffect
	ld b, a
	ld a, [de]
	sub [hl] ; fix: use sub instead of cp so we can check the high byte difference
	inc de
	inc hl
	ld c, a  ; c = high byte difference (0 if equal)
	ld a, [de]
	sbc [hl]
	or c     ; Z only if both high and low byte differences are 0
	jp z, .failed ; no effect if user's HP is already at its maximum
	ld a, b
	cp REST
	jr nz, .healHP
	push hl
	push de
	push af
	ld c, 50
	call DelayFrames
	ld hl, wBattleMonStatus
	ldh a, [hWhoseTurn]
	and a
	jr z, .restEffect
	ld hl, wEnemyMonStatus
.restEffect
	ld a, [hl]
	and a
	push af    ; save Z flag (was the mon already statused before Rest?)
	ld [hl], 2 ; clear status and set number of turns asleep to 2
; Reset Toxic counter and BADLY_POISONED flag (prevents stale N value
; from escalating subsequent poison/burn/Leech Seed damage after waking).
	push hl
	push de
	ld hl, wPlayerBattleStatus3
	ld de, wPlayerToxicCounter
	ldh a, [hWhoseTurn]
	and a
	jr z, .resetToxicPlayer
	ld hl, wEnemyBattleStatus3
	ld de, wEnemyToxicCounter
.resetToxicPlayer
	res BADLY_POISONED, [hl]
	xor a
	ld [de], a
	pop de
	pop hl
	pop af     ; restore Z flag from original status check
	ld hl, StartedSleepingEffect ; if mon didn't have an status
	jr z, .printRestText
	ld hl, FellAsleepBecameHealthyText ; if mon had an status
.printRestText
	call PrintText
	pop af
	pop de
	pop hl
.healHP
	ld a, [hld]
	ld [wHPBarMaxHP], a
	ld c, a
	ld a, [hl]
	ld [wHPBarMaxHP+1], a
	ld b, a
	jr z, .gotHPAmountToHeal
; Recover and Softboiled only heal for half the mon's max HP
	srl b
	rr c
.gotHPAmountToHeal
; update HP
	ld a, [de]
	ld [wHPBarOldHP], a
	add c
	ld [de], a
	ld [wHPBarNewHP], a
	dec de
	ld a, [de]
	ld [wHPBarOldHP+1], a
	adc b
	ld [de], a
	ld [wHPBarNewHP+1], a
	inc hl
	inc de
	ld a, [de]
	dec de
	sub [hl]
	dec hl
	ld a, [de]
	sbc [hl]
	jr c, .playAnim
; copy max HP to current HP if an overflow occurred
	ld a, [hli]
	ld [de], a
	ld [wHPBarNewHP+1], a
	inc de
	ld a, [hl]
	ld [de], a
	ld [wHPBarNewHP], a
.playAnim
	ld hl, PlayCurrentMoveAnimation
	call EffectCallBattleCore
	ldh a, [hWhoseTurn]
	and a
	hlcoord 10, 9
	ld a, $1
	jr z, .updateHPBar
	hlcoord 2, 2
	xor a
.updateHPBar
	ld [wHPBarType], a
	predef UpdateHPBar2
	ld hl, DrawHUDsAndHPBars
	call EffectCallBattleCore
	ld hl, RegainedHealthText
	jp PrintText
.failed
	ld c, 50
	call DelayFrames
	ld hl, PrintButItFailedText_
	jp EffectCallBattleCore

StartedSleepingEffect:
	text_far _StartedSleepingEffect
	text_end

FellAsleepBecameHealthyText:
	text_far _FellAsleepBecameHealthyText
	text_end

RegainedHealthText:
	text_far _RegainedHealthText
	text_end
