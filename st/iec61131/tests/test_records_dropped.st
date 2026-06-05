FUNCTION_BLOCK OPENOT_TestProducerRecordsDropped
VAR_OUTPUT
    Passed : BOOL;
    MismatchIndex : UINT;
    ActualLength : UINT;
    ExpectedLength : UINT;
END_VAR
VAR
    Encoder : OPENOT_EncodeProducerRecordsDropped;
    BadBounds : OPENOT_EncodeProducerRecordsDropped;
    BadCount : OPENOT_EncodeProducerRecordsDropped;
    Expected : ARRAY[0..75] OF BYTE := [
        (* EXPECTED_RECORDS_DROPPED_BEGIN *)
        BYTE#16#4F, BYTE#16#4F, BYTE#16#54, BYTE#16#32, BYTE#16#4C, BYTE#16#00, BYTE#16#08, BYTE#16#00,
        BYTE#16#00, BYTE#16#00, BYTE#16#00, BYTE#16#00, BYTE#16#00, BYTE#16#00, BYTE#16#00, BYTE#16#00,
        BYTE#16#09, BYTE#16#00, BYTE#16#00, BYTE#16#00, BYTE#16#00, BYTE#16#00, BYTE#16#00, BYTE#16#00,
        BYTE#16#2C, BYTE#16#00, BYTE#16#00, BYTE#16#00, BYTE#16#00, BYTE#16#00, BYTE#16#00, BYTE#16#00,
        BYTE#16#2A, BYTE#16#00, BYTE#16#00, BYTE#16#00, BYTE#16#04, BYTE#16#01, BYTE#16#00, BYTE#16#00,
        BYTE#16#16, BYTE#16#00, BYTE#16#05, BYTE#16#04, BYTE#16#05, BYTE#16#00, BYTE#16#00, BYTE#16#00,
        BYTE#16#17, BYTE#16#00, BYTE#16#07, BYTE#16#08, BYTE#16#64, BYTE#16#00, BYTE#16#00, BYTE#16#00,
        BYTE#16#00, BYTE#16#00, BYTE#16#00, BYTE#16#00, BYTE#16#18, BYTE#16#00, BYTE#16#07, BYTE#16#08,
        BYTE#16#68, BYTE#16#00, BYTE#16#00, BYTE#16#00, BYTE#16#00, BYTE#16#00, BYTE#16#00, BYTE#16#00,
        BYTE#16#57, BYTE#16#48, BYTE#16#CC, BYTE#16#E3
        (* EXPECTED_RECORDS_DROPPED_END *)
    ];
    Index : UINT;
END_VAR

Encoder(
    RunId := ULINT#9,
    SourceId := UDINT#42,
    FirstSeq := ULINT#100,
    LastSeq := ULINT#104,
    Seq := ULINT#44,
    SourceTime := ULINT#0);
BadBounds(
    RunId := ULINT#9,
    SourceId := UDINT#42,
    FirstSeq := ULINT#5,
    LastSeq := ULINT#4,
    Seq := ULINT#44,
    SourceTime := ULINT#0);
BadCount(
    RunId := ULINT#9,
    SourceId := UDINT#42,
    FirstSeq := ULINT#0,
    LastSeq := ULINT#4294967295,
    Seq := ULINT#44,
    SourceTime := ULINT#0);

ActualLength := Encoder.Length;
ExpectedLength := UINT#76;
Passed :=
    Encoder.Done AND
    (NOT Encoder.Error) AND
    BadBounds.Error AND
    (NOT BadBounds.Done) AND
    (BadBounds.Length = UINT#0) AND
    BadCount.Error AND
    (NOT BadCount.Done) AND
    (BadCount.Length = UINT#0) AND
    (ActualLength = ExpectedLength);
MismatchIndex := UINT#65535;

IF Passed THEN
    FOR Index := UINT#0 TO ExpectedLength - UINT#1 DO
        IF (Encoder.Buffer[Index] <> Expected[Index]) AND (MismatchIndex = UINT#65535) THEN
            Passed := FALSE;
            MismatchIndex := Index;
        END_IF;
    END_FOR;
END_IF;
END_FUNCTION_BLOCK
