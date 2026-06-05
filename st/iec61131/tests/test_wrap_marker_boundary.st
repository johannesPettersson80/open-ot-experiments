FUNCTION_BLOCK OPENOT_EncodeMinimalMessageRecord
VAR_INPUT
    SourceId : UDINT;
    Seq : ULINT;
END_VAR
VAR_OUTPUT
    Buffer : OPENOT_BYTE_BUFFER;
    Length : UINT;
END_VAR
VAR
    Pos : UINT;
    Crc : DWORD;
END_VAR

Pos := OPENOT_WriteHeader(
    TotalLength := UINT#44,
    Flags := UINT#16#0008,
    SourceTime := ULINT#1780000000000000000 + Seq,
    RunId := ULINT#1,
    Seq := Seq,
    SourceId := SourceId,
    EventTypeId := UDINT#16#0003,
    Buffer := Buffer);

Crc := OPENOT_Crc32c(Data := Buffer, Length := Pos);
Pos := OPENOT_WriteDWordLE(Offset := Pos, Value := Crc, Buffer := Buffer);
Length := Pos;
END_FUNCTION_BLOCK

FUNCTION_BLOCK OPENOT_EncodeMinimalMessageWithUIntSlot
VAR_INPUT
    SourceId : UDINT;
    Seq : ULINT;
END_VAR
VAR_OUTPUT
    Buffer : OPENOT_BYTE_BUFFER;
    Length : UINT;
END_VAR
VAR
    Pos : UINT;
    Crc : DWORD;
END_VAR

Pos := OPENOT_WriteHeader(
    TotalLength := UINT#52,
    Flags := UINT#16#0008,
    SourceTime := ULINT#1780000000000000000 + Seq,
    RunId := ULINT#1,
    Seq := Seq,
    SourceId := SourceId,
    EventTypeId := UDINT#16#0003,
    Buffer := Buffer);

Pos := OPENOT_WriteSlotHeader(
    Offset := Pos,
    Key := UINT#16#1000,
    TypeTag := BYTE#16#03,
    PayloadLength := BYTE#2,
    Buffer := Buffer);
Pos := OPENOT_WriteU16LE(Offset := Pos, Value := UINT#16#2222, Buffer := Buffer);
Pos := OPENOT_WriteByte(Offset := Pos, Value := BYTE#16#00, Buffer := Buffer);
Pos := OPENOT_WriteByte(Offset := Pos, Value := BYTE#16#00, Buffer := Buffer);

Crc := OPENOT_Crc32c(Data := Buffer, Length := Pos);
Pos := OPENOT_WriteDWordLE(Offset := Pos, Value := Crc, Buffer := Buffer);
Length := Pos;
END_FUNCTION_BLOCK

FUNCTION_BLOCK OPENOT_TestWrapMarkerBoundary
VAR_OUTPUT
    Passed : BOOL;
    MismatchIndex : UINT;
    ActualLength : UINT;
    ExpectedLength : UINT;
    ActualHeadAbs : ULINT;
    ActualOldestAbs : ULINT;
    ActualLostCount : ULINT;
END_VAR
VAR
    RingWriter : OPENOT_Ring128;
    Record0 : OPENOT_EncodeMinimalMessageRecord;
    Record1 : OPENOT_EncodeMinimalMessageRecord;
    Record2 : OPENOT_EncodeMinimalMessageWithUIntSlot;
    Expected : ARRAY[0..127] OF BYTE := [
        (* EXPECTED_WRAP_BEGIN *)
        BYTE#16#4F, BYTE#16#4F, BYTE#16#54, BYTE#16#32, BYTE#16#34, BYTE#16#00, BYTE#16#08, BYTE#16#00,
        BYTE#16#02, BYTE#16#00, BYTE#16#32, BYTE#16#B4, BYTE#16#91, BYTE#16#D4, BYTE#16#B3, BYTE#16#18,
        BYTE#16#01, BYTE#16#00, BYTE#16#00, BYTE#16#00, BYTE#16#00, BYTE#16#00, BYTE#16#00, BYTE#16#00,
        BYTE#16#02, BYTE#16#00, BYTE#16#00, BYTE#16#00, BYTE#16#00, BYTE#16#00, BYTE#16#00, BYTE#16#00,
        BYTE#16#51, BYTE#16#00, BYTE#16#00, BYTE#16#00, BYTE#16#03, BYTE#16#00, BYTE#16#00, BYTE#16#00,
        BYTE#16#00, BYTE#16#10, BYTE#16#03, BYTE#16#02, BYTE#16#22, BYTE#16#22, BYTE#16#00, BYTE#16#00,
        BYTE#16#BB, BYTE#16#89, BYTE#16#14, BYTE#16#C3, BYTE#16#01, BYTE#16#00, BYTE#16#32, BYTE#16#B4,
        BYTE#16#91, BYTE#16#D4, BYTE#16#B3, BYTE#16#18, BYTE#16#01, BYTE#16#00, BYTE#16#00, BYTE#16#00,
        BYTE#16#00, BYTE#16#00, BYTE#16#00, BYTE#16#00, BYTE#16#01, BYTE#16#00, BYTE#16#00, BYTE#16#00,
        BYTE#16#00, BYTE#16#00, BYTE#16#00, BYTE#16#00, BYTE#16#51, BYTE#16#00, BYTE#16#00, BYTE#16#00,
        BYTE#16#03, BYTE#16#00, BYTE#16#00, BYTE#16#00, BYTE#16#1F, BYTE#16#24, BYTE#16#45, BYTE#16#20,
        BYTE#16#00, BYTE#16#00, BYTE#16#00, BYTE#16#00, BYTE#16#00, BYTE#16#00, BYTE#16#00, BYTE#16#00,
        BYTE#16#00, BYTE#16#00, BYTE#16#00, BYTE#16#00, BYTE#16#00, BYTE#16#00, BYTE#16#00, BYTE#16#00,
        BYTE#16#00, BYTE#16#00, BYTE#16#00, BYTE#16#00, BYTE#16#00, BYTE#16#00, BYTE#16#00, BYTE#16#00,
        BYTE#16#00, BYTE#16#00, BYTE#16#00, BYTE#16#00, BYTE#16#00, BYTE#16#00, BYTE#16#00, BYTE#16#00,
        BYTE#16#00, BYTE#16#00, BYTE#16#00, BYTE#16#00, BYTE#16#00, BYTE#16#00, BYTE#16#00, BYTE#16#00
        (* EXPECTED_WRAP_END *)
    ];
    Index : UINT;
    Ran : BOOL;
END_VAR

IF NOT Ran THEN
    Record0(SourceId := UDINT#81, Seq := ULINT#0);
    RingWriter(
        Execute := TRUE,
        Record := Record0.Buffer,
        Length := Record0.Length,
        RunId := ULINT#1,
        SourceId := UDINT#81,
        Seq := ULINT#0);

    Record1(SourceId := UDINT#81, Seq := ULINT#1);
    RingWriter(
        Execute := TRUE,
        Record := Record1.Buffer,
        Length := Record1.Length,
        RunId := ULINT#1,
        SourceId := UDINT#81,
        Seq := ULINT#1);

    Record2(SourceId := UDINT#81, Seq := ULINT#2);
    RingWriter(
        Execute := TRUE,
        Record := Record2.Buffer,
        Length := Record2.Length,
        RunId := ULINT#1,
        SourceId := UDINT#81,
        Seq := ULINT#2);

    Ran := TRUE;
END_IF;

ActualLength := UINT#128;
ExpectedLength := UINT#128;
ActualHeadAbs := RingWriter.HeadAbs;
ActualOldestAbs := RingWriter.OldestAbs;
ActualLostCount := RingWriter.LostCount;

Passed :=
    (Record0.Length = UINT#44) AND
    (Record1.Length = UINT#44) AND
    (Record2.Length = UINT#52) AND
    (RingWriter.HeadAbs = ULINT#180) AND
    (RingWriter.OldestAbs = ULINT#128) AND
    (RingWriter.LostCount = ULINT#2) AND
    (NOT RingWriter.Error);
MismatchIndex := UINT#65535;

IF Passed THEN
    FOR Index := UINT#0 TO ExpectedLength - UINT#1 DO
        IF (RingWriter.Ring[Index] <> Expected[Index]) AND (MismatchIndex = UINT#65535) THEN
            Passed := FALSE;
            MismatchIndex := Index;
        END_IF;
    END_FOR;
END_IF;
END_FUNCTION_BLOCK
