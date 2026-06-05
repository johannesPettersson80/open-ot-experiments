FUNCTION_BLOCK OPENOT_EncodeFormationMinimalMessage
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

FUNCTION_BLOCK OPENOT_TestLossRangeFormation
VAR_OUTPUT
    Passed : BOOL;
    FoundSource : BOOL;
    ActualFirstSeq : ULINT;
    ActualLastSeq : ULINT;
    ActualCount : ULINT;
    ActualLossRangeCount : UINT;
    ActualLostCount : ULINT;
END_VAR
VAR
    Producer : OPENOT_Ring256Producer;
    Encoder : OPENOT_EncodeFormationMinimalMessage;
    SeqIndex : UINT;
    LossIndex : UINT;
    SeqValue : ULINT;
    Ran : BOOL;
END_VAR

IF NOT Ran THEN
    FOR SeqIndex := UINT#0 TO UINT#4 DO
        SeqValue := UINT_TO_ULINT(SeqIndex);
        Encoder(SourceId := UDINT#88, Seq := SeqValue);
        Producer(
            Execute := TRUE,
            Record := Encoder.Buffer,
            Length := Encoder.Length,
            RunId := ULINT#1,
            SourceId := UDINT#88,
            Seq := SeqValue);
    END_FOR;

    FOR SeqIndex := UINT#0 TO UINT#79 DO
        SeqValue := UINT_TO_ULINT(SeqIndex);
        Encoder(SourceId := UDINT#99, Seq := SeqValue);
        Producer(
            Execute := TRUE,
            Record := Encoder.Buffer,
            Length := Encoder.Length,
            RunId := ULINT#1,
            SourceId := UDINT#99,
            Seq := SeqValue);
    END_FOR;

    Ran := TRUE;
END_IF;

FoundSource := FALSE;
ActualFirstSeq := ULINT#0;
ActualLastSeq := ULINT#0;
ActualCount := ULINT#0;
ActualLossRangeCount := Producer.LossRangeCount;
ActualLostCount := Producer.LostCount;

IF Producer.LossRangeCount > UINT#0 THEN
    FOR LossIndex := UINT#0 TO Producer.LossRangeCount - UINT#1 DO
        IF (Producer.LossRanges[LossIndex].RunId = ULINT#1) AND (Producer.LossRanges[LossIndex].SourceId = UDINT#88) AND (NOT FoundSource) THEN
            FoundSource := TRUE;
            ActualFirstSeq := Producer.LossRanges[LossIndex].FirstSeq;
            ActualLastSeq := Producer.LossRanges[LossIndex].LastSeq;
            ActualCount := (ActualLastSeq - ActualFirstSeq) + ULINT#1;
        END_IF;
    END_FOR;
END_IF;

Passed :=
    FoundSource AND
    (ActualFirstSeq = ULINT#0) AND
    (ActualLastSeq = ULINT#4) AND
    (ActualCount = ULINT#5) AND
    (NOT Producer.LossRangeOverflow) AND
    (NOT Producer.Error);
END_FUNCTION_BLOCK
