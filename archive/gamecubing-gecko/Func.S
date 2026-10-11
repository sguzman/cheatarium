#80273AE8
bctrl	
lbz	r0, 0x02B8 (r3)
mr	r30, r3
cmpwi	r0, 3
#bne-	 0x80411C34
lwz	r12, 0 (r3)
lwz	r12, 0x00A8 (r12)
mtctr	r12
bctrl	
rlwinm.	r0, r3, 0, 24, 31
beq-	 0x80411C34
mr	r3, r30
lwz	r12, 0 (r30)
lwz	r12, 0x01C0 (r12)
mtctr	r12
bctrl	
rlwinm.	r0, r3, 0, 24, 31
beq-	0x80411C34
mr	r3, r30
bl	0x8019EAB4
80411b14: rlwinm.	r0, r3, 0, 24, 31
80411b18: bne-	 ->0x80411C34
80411b1c: lwz	r0, 0x00F4 (r30)
80411b20: cmplw	r0, r31
80411b24: beq-	 ->0x80411C34
80411b28: lwz	r0, 0x00C8 (r30)
80411b2c: cmplwi	r0, 0
80411b30: beq-	 ->0x80411C34
80411b34: mr	r4, r30
80411b38: addi	r3, sp, 32
80411b3c: lwz	r12, 0 (r30)
80411b40: lwz	r12, 0x0008 (r12)
80411b44: mtctr	r12
80411b48: bctrl	
80411b4c: mr	r4, r31
80411b50: lfs	f2, 0x0020 (sp)
80411b54: lwz	r12, 0 (r31)
80411b58: addi	r3, sp, 44
80411b5c: lfs	f1, 0x0024 (sp)
80411b60: lfs	f0, 0x0028 (sp)
80411b64: lwz	r12, 0x0008 (r12)
80411b68: stfs	f2, 0x0008 (sp)
80411b6c: stfs	f1, 0x000C (sp)
80411b70: stfs	f0, 0x0010 (sp)
80411b74: mtctr	r12
80411b78: bctrl	
80411b7c: lfs	f5, 0x002C (sp)
80411b80: lis	r3, 0x8051
80411b84: lfs	f3, 0x0034 (sp)
80411b88: addi	r3, r3, 21696
80411b8c: lfs	f1, 0x0008 (sp)
80411b90: lfs	f0, 0x0010 (sp)
80411b94: lfs	f4, 0x0030 (sp)
80411b98: fsubs	f1, f1, f5
80411b9c: fsubs	f2, f0, f3
80411ba0: stfs	f5, 0x0014 (sp)
80411ba4: stfs	f4, 0x0018 (sp)
80411ba8: stfs	f3, 0x001C (sp)
80411bac: bl	->0x80035108
80411bb0: bl	->0x80412760
80411bb4: lwz	r12, 0 (r31)
80411bb8: fmr	f29, f1
80411bbc: mr	r3, r31
80411bc0: lwz	r12, 0x0064 (r12)
80411bc4: mtctr	r12
80411bc8: bctrl	
80411bcc: fmr	f2, f1
80411bd0: fmr	f1, f29
80411bd4: bl	->0x8041278C
80411bd8: fabs	f0, f1
80411bdc: frsp	f0, f0
80411be0: fcmpo	cr0, f0, f31
80411be4: cror	2, 0, 2
80411be8: bne-	 ->0x80411C34
80411bec: mr	r4, r30
80411bf0: addi	r3, sp, 56
80411bf4: lwz	r12, 0 (r30)
80411bf8: lwz	r12, 0x0008 (r12)
80411bfc: mtctr	r12
80411c00: bctrl	
80411c04: lfs	f1, 0x0194 (r31)
80411c08: lfs	f0, 0x0040 (sp)
80411c0c: lfs	f2, 0x018C (r31)
80411c10: fsubs	f1, f1, f0
80411c14: lfs	f0, 0x0038 (sp)
80411c18: fsubs	f2, f2, f0
80411c1c: fmuls	f0, f1, f1
80411c20: fmadds	f0, f2, f2, f0
80411c24: fcmpo	cr0, f0, f30
80411c28: bge-	 ->0x80411C34
80411c2c: mr	r3, r30
80411c30: b	->0x80411D14
80411c34: lwz	r0, 0x0050 (sp)
80411c38: cmplwi	r0, 0
80411c3c: bne-	 ->0x80411C60
80411c40: lwz	r3, 0x004C (sp)
80411c44: lwz	r4, 0x0048 (sp)
80411c48: lwz	r12, 0 (r3)
80411c4c: lwz	r12, 0x0014 (r12)
80411c50: mtctr	r12
80411c54: bctrl	
80411c58: stw	r3, 0x0048 (sp)
80411c5c: b	->0x80411CF0
80411c60: lwz	r3, 0x004C (sp)
80411c64: lwz	r4, 0x0048 (sp)
80411c68: lwz	r12, 0 (r3)
80411c6c: lwz	r12, 0x0014 (r12)
80411c70: mtctr	r12
80411c74: bctrl	
80411c78: stw	r3, 0x0048 (sp)
80411c7c: b	->0x80411CD4
80411c80: lwz	r3, 0x004C (sp)
80411c84: lwz	r4, 0x0048 (sp)
80411c88: lwz	r12, 0 (r3)
80411c8c: lwz	r12, 0x0020 (r12)
80411c90: mtctr	r12
80411c94: bctrl	
80411c98: mr	r4, r3
80411c9c: lwz	r3, 0x0050 (sp)
80411ca0: lwz	r12, 0 (r3)
80411ca4: lwz	r12, 0x0008 (r12)
80411ca8: mtctr	r12
80411cac: bctrl	
80411cb0: rlwinm.	r0, r3, 0, 24, 31 (000000ff)
80411cb4: bne-	 ->0x80411CF0
80411cb8: lwz	r3, 0x004C (sp)
80411cbc: lwz	r4, 0x0048 (sp)
80411cc0: lwz	r12, 0 (r3)
80411cc4: lwz	r12, 0x0014 (r12)
80411cc8: mtctr	r12
80411ccc: bctrl	
80411cd0: stw	r3, 0x0048 (sp)
80411cd4: lwz	r12, 0x0044 (sp)
80411cd8: addi	r3, sp, 68
80411cdc: lwz	r12, 0x0010 (r12)
80411ce0: mtctr	r12
80411ce4: bctrl	
80411ce8: rlwinm.	r0, r3, 0, 24, 31 (000000ff)
80411cec: beq+	 ->0x80411C80
80411cf0: lwz	r3, 0x004C (sp)
80411cf4: lwz	r12, 0 (r3)
80411cf8: lwz	r12, 0x001C (r12)
80411cfc: mtctr	r12
bctrl	
lwz	r4, 0x0048 (sp)
cmplw	r4, r3
bne+	 0x80411AB4
li	r3, 0
psq_l	f31, 0x0088(r1), 0, 0
lfd	f31, 0x0080 (sp)
psq_l	f30, 0x0078(r1), 0, 0
lfd	f30, 0x0070 (sp)
psq_l	f29, 0x0068(r1), 0, 0
lfd	f29, 0x0060 (sp)
lwz	r31, 0x005C (sp)
lwz	r0, 0x0094 (sp)
lwz	r30, 0x0058 (sp)
mtlr	r0
addi	sp, sp, 144
blr	
