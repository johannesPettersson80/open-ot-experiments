FUNCTION_BLOCK OPENOT_TestConformantConditionLifecycle
VAR_OUTPUT
    Passed : BOOL;
    AckPassed : BOOL;
    ShelvedPassed : BOOL;
    SuppressedPassed : BOOL;
    OutOfServicePassed : BOOL;
    MismatchIndex : UINT;
END_VAR
VAR
    AckEncoder : OPENOT_EncodeConformantConditionAcknowledged;
    ShelvedEncoder : OPENOT_EncodeConformantConditionShelved;
    SuppressedEncoder : OPENOT_EncodeConformantConditionSuppressed;
    OutOfServiceEncoder : OPENOT_EncodeConformantConditionOutOfService;
    ExpectedAck : ARRAY[0..75] OF BYTE := [
        BYTE#16#4F, BYTE#16#4F, BYTE#16#54, BYTE#16#32, BYTE#16#4C, BYTE#16#00, BYTE#16#08, BYTE#16#00,
        BYTE#16#A0, BYTE#16#CA, BYTE#16#9A, BYTE#16#3B, BYTE#16#00, BYTE#16#00, BYTE#16#00, BYTE#16#00,
        BYTE#16#01, BYTE#16#00, BYTE#16#00, BYTE#16#00, BYTE#16#00, BYTE#16#00, BYTE#16#00, BYTE#16#00,
        BYTE#16#11, BYTE#16#00, BYTE#16#00, BYTE#16#00, BYTE#16#00, BYTE#16#00, BYTE#16#00, BYTE#16#00,
        BYTE#16#42, BYTE#16#00, BYTE#16#00, BYTE#16#00, BYTE#16#02, BYTE#16#02, BYTE#16#00, BYTE#16#00,
        BYTE#16#05, BYTE#16#00, BYTE#16#05, BYTE#16#04, BYTE#16#29, BYTE#16#23, BYTE#16#00, BYTE#16#00,
        BYTE#16#07, BYTE#16#00, BYTE#16#05, BYTE#16#04, BYTE#16#4D, BYTE#16#00, BYTE#16#00, BYTE#16#00,
        BYTE#16#1D, BYTE#16#00, BYTE#16#0C, BYTE#16#0A, BYTE#16#6F, BYTE#16#70, BYTE#16#65, BYTE#16#72,
        BYTE#16#61, BYTE#16#74, BYTE#16#6F, BYTE#16#72, BYTE#16#2D, BYTE#16#61, BYTE#16#00, BYTE#16#00,
        BYTE#16#FA, BYTE#16#EA, BYTE#16#82, BYTE#16#EC
    ];
    ExpectedShelved : ARRAY[0..83] OF BYTE := [
        BYTE#16#4F, BYTE#16#4F, BYTE#16#54, BYTE#16#32, BYTE#16#54, BYTE#16#00, BYTE#16#08, BYTE#16#00,
        BYTE#16#AA, BYTE#16#CA, BYTE#16#9A, BYTE#16#3B, BYTE#16#00, BYTE#16#00, BYTE#16#00, BYTE#16#00,
        BYTE#16#01, BYTE#16#00, BYTE#16#00, BYTE#16#00, BYTE#16#00, BYTE#16#00, BYTE#16#00, BYTE#16#00,
        BYTE#16#12, BYTE#16#00, BYTE#16#00, BYTE#16#00, BYTE#16#00, BYTE#16#00, BYTE#16#00, BYTE#16#00,
        BYTE#16#42, BYTE#16#00, BYTE#16#00, BYTE#16#00, BYTE#16#04, BYTE#16#02, BYTE#16#00, BYTE#16#00,
        BYTE#16#05, BYTE#16#00, BYTE#16#05, BYTE#16#04, BYTE#16#29, BYTE#16#23, BYTE#16#00, BYTE#16#00,
        BYTE#16#07, BYTE#16#00, BYTE#16#05, BYTE#16#04, BYTE#16#4D, BYTE#16#00, BYTE#16#00, BYTE#16#00,
        BYTE#16#1D, BYTE#16#00, BYTE#16#0C, BYTE#16#0A, BYTE#16#6F, BYTE#16#70, BYTE#16#65, BYTE#16#72,
        BYTE#16#61, BYTE#16#74, BYTE#16#6F, BYTE#16#72, BYTE#16#2D, BYTE#16#61, BYTE#16#00, BYTE#16#00,
        BYTE#16#1E, BYTE#16#00, BYTE#16#05, BYTE#16#04, BYTE#16#2C, BYTE#16#01, BYTE#16#00, BYTE#16#00,
        BYTE#16#E5, BYTE#16#3E, BYTE#16#92, BYTE#16#BD
    ];
    ExpectedSuppressed : ARRAY[0..67] OF BYTE := [
        BYTE#16#4F, BYTE#16#4F, BYTE#16#54, BYTE#16#32, BYTE#16#44, BYTE#16#00, BYTE#16#08, BYTE#16#00,
        BYTE#16#B4, BYTE#16#CA, BYTE#16#9A, BYTE#16#3B, BYTE#16#00, BYTE#16#00, BYTE#16#00, BYTE#16#00,
        BYTE#16#01, BYTE#16#00, BYTE#16#00, BYTE#16#00, BYTE#16#00, BYTE#16#00, BYTE#16#00, BYTE#16#00,
        BYTE#16#13, BYTE#16#00, BYTE#16#00, BYTE#16#00, BYTE#16#00, BYTE#16#00, BYTE#16#00, BYTE#16#00,
        BYTE#16#42, BYTE#16#00, BYTE#16#00, BYTE#16#00, BYTE#16#06, BYTE#16#02, BYTE#16#00, BYTE#16#00,
        BYTE#16#05, BYTE#16#00, BYTE#16#05, BYTE#16#04, BYTE#16#29, BYTE#16#23, BYTE#16#00, BYTE#16#00,
        BYTE#16#1F, BYTE#16#00, BYTE#16#0C, BYTE#16#0B, BYTE#16#6D, BYTE#16#61, BYTE#16#69, BYTE#16#6E,
        BYTE#16#74, BYTE#16#65, BYTE#16#6E, BYTE#16#61, BYTE#16#6E, BYTE#16#63, BYTE#16#65, BYTE#16#00,
        BYTE#16#17, BYTE#16#3E, BYTE#16#4F, BYTE#16#63
    ];
    ExpectedOutOfService : ARRAY[0..67] OF BYTE := [
        BYTE#16#4F, BYTE#16#4F, BYTE#16#54, BYTE#16#32, BYTE#16#44, BYTE#16#00, BYTE#16#08, BYTE#16#00,
        BYTE#16#BE, BYTE#16#CA, BYTE#16#9A, BYTE#16#3B, BYTE#16#00, BYTE#16#00, BYTE#16#00, BYTE#16#00,
        BYTE#16#01, BYTE#16#00, BYTE#16#00, BYTE#16#00, BYTE#16#00, BYTE#16#00, BYTE#16#00, BYTE#16#00,
        BYTE#16#14, BYTE#16#00, BYTE#16#00, BYTE#16#00, BYTE#16#00, BYTE#16#00, BYTE#16#00, BYTE#16#00,
        BYTE#16#42, BYTE#16#00, BYTE#16#00, BYTE#16#00, BYTE#16#08, BYTE#16#02, BYTE#16#00, BYTE#16#00,
        BYTE#16#05, BYTE#16#00, BYTE#16#05, BYTE#16#04, BYTE#16#29, BYTE#16#23, BYTE#16#00, BYTE#16#00,
        BYTE#16#1D, BYTE#16#00, BYTE#16#0C, BYTE#16#0A, BYTE#16#6F, BYTE#16#70, BYTE#16#65, BYTE#16#72,
        BYTE#16#61, BYTE#16#74, BYTE#16#6F, BYTE#16#72, BYTE#16#2D, BYTE#16#61, BYTE#16#00, BYTE#16#00,
        BYTE#16#9D, BYTE#16#5C, BYTE#16#A4, BYTE#16#01
    ];
    Index : UINT;
END_VAR

AckEncoder();
ShelvedEncoder();
SuppressedEncoder();
OutOfServiceEncoder();

AckPassed := AckEncoder.Length = UINT#76;
ShelvedPassed := ShelvedEncoder.Length = UINT#84;
SuppressedPassed := SuppressedEncoder.Length = UINT#68;
OutOfServicePassed := OutOfServiceEncoder.Length = UINT#68;
MismatchIndex := UINT#65535;

IF AckPassed THEN
    FOR Index := UINT#0 TO UINT#75 DO
        IF (AckEncoder.Buffer[Index] <> ExpectedAck[Index]) AND (MismatchIndex = UINT#65535) THEN
            AckPassed := FALSE;
            MismatchIndex := Index;
        END_IF;
    END_FOR;
END_IF;

IF ShelvedPassed THEN
    FOR Index := UINT#0 TO UINT#83 DO
        IF (ShelvedEncoder.Buffer[Index] <> ExpectedShelved[Index]) AND (MismatchIndex = UINT#65535) THEN
            ShelvedPassed := FALSE;
            MismatchIndex := Index;
        END_IF;
    END_FOR;
END_IF;

IF SuppressedPassed THEN
    FOR Index := UINT#0 TO UINT#67 DO
        IF (SuppressedEncoder.Buffer[Index] <> ExpectedSuppressed[Index]) AND (MismatchIndex = UINT#65535) THEN
            SuppressedPassed := FALSE;
            MismatchIndex := Index;
        END_IF;
    END_FOR;
END_IF;

IF OutOfServicePassed THEN
    FOR Index := UINT#0 TO UINT#67 DO
        IF (OutOfServiceEncoder.Buffer[Index] <> ExpectedOutOfService[Index]) AND (MismatchIndex = UINT#65535) THEN
            OutOfServicePassed := FALSE;
            MismatchIndex := Index;
        END_IF;
    END_FOR;
END_IF;

Passed := AckPassed AND ShelvedPassed AND SuppressedPassed AND OutOfServicePassed;
END_FUNCTION_BLOCK
