FUNCTION_BLOCK OPENOT_EncodeInterleavedMinimalMessage
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

FUNCTION_BLOCK OPENOT_TestLossRangeInterleaved
VAR_OUTPUT
    Passed : BOOL;
    ActualLossRangeCount : UINT;
    ActualLostCount : ULINT;
END_VAR
VAR
    Producer : OPENOT_Ring256Producer;
    Encoder : OPENOT_EncodeInterleavedMinimalMessage;
    Ran : BOOL;
END_VAR

IF NOT Ran THEN
    Encoder(SourceId := UDINT#10, Seq := ULINT#0);
    Producer(
        Execute := TRUE,
        Record := Encoder.Buffer,
        Length := Encoder.Length,
        RunId := ULINT#1,
        SourceId := UDINT#10,
        Seq := ULINT#0);
    Encoder(SourceId := UDINT#20, Seq := ULINT#0);
    Producer(
        Execute := TRUE,
        Record := Encoder.Buffer,
        Length := Encoder.Length,
        RunId := ULINT#1,
        SourceId := UDINT#20,
        Seq := ULINT#0);
    Encoder(SourceId := UDINT#10, Seq := ULINT#1);
    Producer(
        Execute := TRUE,
        Record := Encoder.Buffer,
        Length := Encoder.Length,
        RunId := ULINT#1,
        SourceId := UDINT#10,
        Seq := ULINT#1);
    Encoder(SourceId := UDINT#20, Seq := ULINT#1);
    Producer(
        Execute := TRUE,
        Record := Encoder.Buffer,
        Length := Encoder.Length,
        RunId := ULINT#1,
        SourceId := UDINT#20,
        Seq := ULINT#1);
    Encoder(SourceId := UDINT#10, Seq := ULINT#2);
    Producer(
        Execute := TRUE,
        Record := Encoder.Buffer,
        Length := Encoder.Length,
        RunId := ULINT#1,
        SourceId := UDINT#10,
        Seq := ULINT#2);
    Encoder(SourceId := UDINT#20, Seq := ULINT#2);
    Producer(
        Execute := TRUE,
        Record := Encoder.Buffer,
        Length := Encoder.Length,
        RunId := ULINT#1,
        SourceId := UDINT#20,
        Seq := ULINT#2);
    Encoder(SourceId := UDINT#10, Seq := ULINT#3);
    Producer(
        Execute := TRUE,
        Record := Encoder.Buffer,
        Length := Encoder.Length,
        RunId := ULINT#1,
        SourceId := UDINT#10,
        Seq := ULINT#3);
    Encoder(SourceId := UDINT#20, Seq := ULINT#3);
    Producer(
        Execute := TRUE,
        Record := Encoder.Buffer,
        Length := Encoder.Length,
        RunId := ULINT#1,
        SourceId := UDINT#20,
        Seq := ULINT#3);
    Encoder(SourceId := UDINT#10, Seq := ULINT#4);
    Producer(
        Execute := TRUE,
        Record := Encoder.Buffer,
        Length := Encoder.Length,
        RunId := ULINT#1,
        SourceId := UDINT#10,
        Seq := ULINT#4);

    Ran := TRUE;
END_IF;

ActualLossRangeCount := Producer.LossRangeCount;
ActualLostCount := Producer.LostCount;

Passed :=
    (Producer.LossRangeCount = UINT#4) AND
    (Producer.LostCount = ULINT#4) AND
    (Producer.LossRanges[0].RunId = ULINT#1) AND
    (Producer.LossRanges[0].SourceId = UDINT#10) AND
    (Producer.LossRanges[0].FirstSeq = ULINT#0) AND
    (Producer.LossRanges[0].LastSeq = ULINT#0) AND
    (Producer.LossRanges[1].RunId = ULINT#1) AND
    (Producer.LossRanges[1].SourceId = UDINT#20) AND
    (Producer.LossRanges[1].FirstSeq = ULINT#0) AND
    (Producer.LossRanges[1].LastSeq = ULINT#0) AND
    (Producer.LossRanges[2].RunId = ULINT#1) AND
    (Producer.LossRanges[2].SourceId = UDINT#10) AND
    (Producer.LossRanges[2].FirstSeq = ULINT#1) AND
    (Producer.LossRanges[2].LastSeq = ULINT#1) AND
    (Producer.LossRanges[3].RunId = ULINT#1) AND
    (Producer.LossRanges[3].SourceId = UDINT#20) AND
    (Producer.LossRanges[3].FirstSeq = ULINT#1) AND
    (Producer.LossRanges[3].LastSeq = ULINT#1) AND
    (NOT Producer.LossRangeOverflow) AND
    (NOT Producer.Error);
END_FUNCTION_BLOCK
