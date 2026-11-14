use super::*;
use serde_json::json;

#[test]
fn test_traverse_empty_object() {
    let template = json!({});
    let result = traverse_and_update(template.clone(), "OldId", "NewId");
    assert_eq!(result, template);
}

#[test]
fn test_update_ref_basic() {
    let template = json!({ "Ref": "OldBucket" });
    let result = traverse_and_update(template, "OldBucket", "NewBucket");
    assert_eq!(result, json!({ "Ref": "NewBucket" }));
}

#[test]
fn test_update_ref_pseudo_parameter() {
    let template = json!({ "Ref": "AWS::Region" });
    let result = traverse_and_update(template.clone(), "AWS::Region", "NewRegion");
    assert_eq!(result, json!({ "Ref": "AWS::Region" })); // Unchanged
}

#[test]
fn test_update_ref_not_matching() {
    let template = json!({ "Ref": "OtherResource" });
    let result = traverse_and_update(template.clone(), "OldBucket", "NewBucket");
    assert_eq!(result, json!({ "Ref": "OtherResource" })); // Unchanged
}

#[test]
fn test_update_getatt_basic() {
    let template = json!({ "Fn::GetAtt": ["OldBucket", "Arn"] });
    let result = traverse_and_update(template, "OldBucket", "NewBucket");
    assert_eq!(result, json!({ "Fn::GetAtt": ["NewBucket", "Arn"] }));
}

#[test]
fn test_update_getatt_not_matching() {
    let template = json!({ "Fn::GetAtt": ["OtherResource", "Arn"] });
    let result = traverse_and_update(template.clone(), "OldBucket", "NewBucket");
    assert_eq!(result, json!({ "Fn::GetAtt": ["OtherResource", "Arn"] }));
}

#[test]
fn test_update_getatt_string_form() {
    let template = json!({ "Fn::GetAtt": "OldBucket.Arn" });
    let result = traverse_and_update(template, "OldBucket", "NewBucket");
    assert_eq!(result, json!({ "Fn::GetAtt": "NewBucket.Arn" }));
}

#[test]
fn test_update_getatt_string_form_not_matching() {
    let template = json!({ "Fn::GetAtt": "OtherResource.Arn" });
    let result = traverse_and_update(template.clone(), "OldBucket", "NewBucket");
    assert_eq!(result, json!({ "Fn::GetAtt": "OtherResource.Arn" }));
}

#[test]
fn test_update_dependson_string() {
    let template = json!({
        "Type": "AWS::EC2::Instance",
        "DependsOn": "OldSecurityGroup",
        "Properties": {}
    });
    let result = traverse_and_update(template, "OldSecurityGroup", "NewSecurityGroup");
    assert_eq!(result["DependsOn"], json!("NewSecurityGroup"));
}

#[test]
fn test_update_dependson_array() {
    let template = json!({
        "Type": "AWS::EC2::Instance",
        "DependsOn": ["OldSG", "OtherResource", "OldBucket"],
        "Properties": {}
    });
    let mut mapping = HashMap::new();
    mapping.insert("OldSG".to_string(), "NewSG".to_string());
    mapping.insert("OldBucket".to_string(), "NewBucket".to_string());

    let mut result = template.clone();
    for (old_id, new_id) in &mapping {
        result = traverse_and_update(result, old_id, new_id);
    }

    assert_eq!(
        result["DependsOn"],
        json!(["NewSG", "OtherResource", "NewBucket"])
    );
}

#[test]
fn test_update_sub_simple() {
    let template = json!({ "Fn::Sub": "arn:aws:s3:::${OldBucket}/*" });
    let result = traverse_and_update(template, "OldBucket", "NewBucket");
    assert_eq!(result, json!({ "Fn::Sub": "arn:aws:s3:::${NewBucket}/*" }));
}

#[test]
fn test_update_sub_multiple_vars() {
    let template = json!({ "Fn::Sub": "${OldBucket}-${OldTable}" });
    let mut result = template.clone();
    result = traverse_and_update(result, "OldBucket", "NewBucket");
    result = traverse_and_update(result, "OldTable", "NewTable");
    assert_eq!(result, json!({ "Fn::Sub": "${NewBucket}-${NewTable}" }));
}

#[test]
fn test_update_sub_no_partial_match() {
    let template = json!({ "Fn::Sub": "${OldBucket}-${OldBucket2}" });
    let result = traverse_and_update(template, "OldBucket", "NewBucket");
    // Should only replace exact matches
    assert_eq!(result, json!({ "Fn::Sub": "${NewBucket}-${OldBucket2}" }));
}

#[test]
fn test_update_sub_with_attribute() {
    let template = json!({ "Fn::Sub": "${OldBucket.Arn}" });
    let result = traverse_and_update(template, "OldBucket", "NewBucket");
    assert_eq!(result, json!({ "Fn::Sub": "${NewBucket.Arn}" }));
}

#[test]
fn test_update_sub_array_form() {
    let template = json!({
        "Fn::Sub": [
            "${BucketName}-suffix",
            {"BucketName": {"Ref": "OldBucket"}}
        ]
    });
    let result = traverse_and_update(template, "OldBucket", "NewBucket");
    assert_eq!(
        result,
        json!({
            "Fn::Sub": [
                "${BucketName}-suffix",
                {"BucketName": {"Ref": "NewBucket"}}
            ]
        })
    );
}

#[test]
fn test_update_sub_array_rename_key() {
    let template = json!({
        "Fn::Sub": [
            "${OldBucket}-suffix",
            {"OldBucket": "some-value"}
        ]
    });
    let result = traverse_and_update(template, "OldBucket", "NewBucket");
    assert_eq!(
        result,
        json!({
            "Fn::Sub": [
                "${NewBucket}-suffix",
                {"NewBucket": "some-value"}
            ]
        })
    );
}

#[test]
fn test_update_sub_array_local_variable() {
    // When a variable is defined in the variable map, the rename should still
    // update both the variable map key AND the template string consistently
    let template = json!({
        "Fn::Sub": [
            "${LocalVar}-${OldBucket}",
            {"LocalVar": "hardcoded-value"}
        ]
    });
    let result = traverse_and_update(template, "LocalVar", "NewLocalVar");
    // Both the template string and the map key should be renamed
    assert_eq!(
        result,
        json!({
            "Fn::Sub": [
                "${NewLocalVar}-${OldBucket}",
                {"NewLocalVar": "hardcoded-value"}
            ]
        })
    );
}

#[test]
fn test_update_sub_escape_syntax() {
    // ${!Literal} should NOT be treated as a reference (it's escaped)
    let template = json!({ "Fn::Sub": "${!Literal}-${OldBucket}" });
    let result = traverse_and_update(template, "Literal", "NewLiteral");
    // Should NOT update the escaped ${!Literal} part
    assert_eq!(result, json!({ "Fn::Sub": "${!Literal}-${OldBucket}" }));
}

#[test]
fn test_nested_structure() {
    let template = json!({
        "Resources": {
            "MyInstance": {
                "Type": "AWS::EC2::Instance",
                "Properties": {
                    "SecurityGroups": [
                        {"Ref": "OldSG"}
                    ]
                }
            }
        }
    });
    let result = traverse_and_update(template, "OldSG", "NewSG");
    assert_eq!(
        result["Resources"]["MyInstance"]["Properties"]["SecurityGroups"][0],
        json!({"Ref": "NewSG"})
    );
}

#[test]
fn test_multiple_references_same_resource() {
    let template = json!({
        "Resources": {
            "Resource1": {
                "Properties": {
                    "Bucket": {"Ref": "OldBucket"},
                    "BucketArn": {"Fn::GetAtt": ["OldBucket", "Arn"]}
                }
            }
        },
        "Outputs": {
            "BucketName": {
                "Value": {"Ref": "OldBucket"}
            }
        }
    });
    let result = traverse_and_update(template, "OldBucket", "NewBucket");

    assert_eq!(
        result["Resources"]["Resource1"]["Properties"]["Bucket"],
        json!({"Ref": "NewBucket"})
    );
    assert_eq!(
        result["Resources"]["Resource1"]["Properties"]["BucketArn"],
        json!({"Fn::GetAtt": ["NewBucket", "Arn"]})
    );
    assert_eq!(
        result["Outputs"]["BucketName"]["Value"],
        json!({"Ref": "NewBucket"})
    );
}

#[test]
fn test_update_template_references_main_api() {
    let template = json!({
        "Resources": {
            "MyLambda": {
                "Type": "AWS::Lambda::Function",
                "Properties": {
                    "Environment": {
                        "Variables": {
                            "BUCKET": {"Ref": "OldBucket"},
                            "TABLE": {"Ref": "OldTable"}
                        }
                    }
                }
            }
        }
    });

    let mut mapping = HashMap::new();
    mapping.insert("OldBucket".to_string(), "NewBucket".to_string());
    mapping.insert("OldTable".to_string(), "NewTable".to_string());

    let result = update_template_references(template, &mapping);

    assert_eq!(
        result["Resources"]["MyLambda"]["Properties"]["Environment"]["Variables"]["BUCKET"],
        json!({"Ref": "NewBucket"})
    );
    assert_eq!(
        result["Resources"]["MyLambda"]["Properties"]["Environment"]["Variables"]["TABLE"],
        json!({"Ref": "NewTable"})
    );
}

#[test]
fn test_find_all_references_ref() {
    let template = json!({
        "Resources": {
            "Lambda": {
                "Type": "AWS::Lambda::Function",
                "Properties": {
                    "Environment": {
                        "Variables": {
                            "BUCKET": {"Ref": "MyBucket"}
                        }
                    }
                }
            },
            "MyBucket": {
                "Type": "AWS::S3::Bucket"
            }
        }
    });

    let references = find_all_references(&template);
    assert!(references.contains_key("Lambda"));
    assert!(references["Lambda"].contains("MyBucket"));
}

#[test]
fn test_find_all_references_getatt() {
    let template = json!({
        "Resources": {
            "Lambda": {
                "Type": "AWS::Lambda::Function",
                "Properties": {
                    "TableArn": {"Fn::GetAtt": ["MyTable", "Arn"]}
                }
            }
        }
    });

    let references = find_all_references(&template);
    assert!(references["Lambda"].contains("MyTable"));
}

#[test]
fn test_find_all_references_dependson() {
    let template = json!({
        "Resources": {
            "Instance": {
                "Type": "AWS::EC2::Instance",
                "DependsOn": "MySecurityGroup",
                "Properties": {}
            }
        }
    });

    let references = find_all_references(&template);
    assert!(references["Instance"].contains("MySecurityGroup"));
}

#[test]
fn test_find_all_references_sub() {
    let template = json!({
        "Resources": {
            "Policy": {
                "Type": "AWS::IAM::Policy",
                "Properties": {
                    "PolicyDocument": {
                        "Statement": [{
                            "Resource": {"Fn::Sub": "arn:aws:s3:::${MyBucket}/*"}
                        }]
                    }
                }
            }
        }
    });

    let references = find_all_references(&template);
    assert!(references["Policy"].contains("MyBucket"));
}

#[test]
fn test_find_all_references_outputs() {
    let template = json!({
        "Resources": {
            "MyBucket": {
                "Type": "AWS::S3::Bucket"
            }
        },
        "Outputs": {
            "BucketName": {
                "Value": {"Ref": "MyBucket"}
            }
        }
    });

    let references = find_all_references(&template);
    assert!(references.contains_key("Outputs"));
    assert!(references["Outputs"].contains("MyBucket"));
}

#[test]
fn test_find_all_references_multiple() {
    let template = json!({
        "Resources": {
            "Lambda": {
                "Type": "AWS::Lambda::Function",
                "DependsOn": ["MyBucket", "MyTable"],
                "Properties": {
                    "Environment": {
                        "Variables": {
                            "BUCKET": {"Ref": "MyBucket"},
                            "TABLE_ARN": {"Fn::GetAtt": ["MyTable", "Arn"]}
                        }
                    },
                    "Role": {"Fn::Sub": "arn:aws:iam::${AWS::AccountId}:role/${MyRole}"}
                }
            }
        }
    });

    let references = find_all_references(&template);
    let lambda_refs = &references["Lambda"];

    assert!(lambda_refs.contains("MyBucket"));
    assert!(lambda_refs.contains("MyTable"));
    assert!(lambda_refs.contains("MyRole"));
    assert!(!lambda_refs.contains("AWS::AccountId")); // Pseudo-parameter should be ignored
}

#[test]
fn test_find_all_references_ignores_pseudo_parameters() {
    let template = json!({
        "Resources": {
            "Resource": {
                "Type": "AWS::S3::Bucket",
                "Properties": {
                    "BucketName": {"Fn::Sub": "${AWS::StackName}-bucket"},
                    "Tags": [{
                        "Key": "Region",
                        "Value": {"Ref": "AWS::Region"}
                    }]
                }
            }
        }
    });

    let references = find_all_references(&template);

    // Pseudo-parameters are no references, so nothing is found
    assert!(!references.contains_key("Resource"));
}

#[test]
fn test_find_all_references_sub_with_variable_map() {
    // When Fn::Sub has a variable map, variables defined in the map
    // should NOT be treated as resource references
    let template = json!({
        "Resources": {
            "Lambda": {
                "Type": "AWS::Lambda::Function",
                "Properties": {
                    "FunctionName": {
                        "Fn::Sub": [
                            "${Prefix}-${BucketName}",
                            {"Prefix": "my-function"}
                        ]
                    },
                    "Environment": {
                        "Variables": {
                            "BUCKET": {"Ref": "MyBucket"}
                        }
                    }
                }
            },
            "MyBucket": {
                "Type": "AWS::S3::Bucket"
            },
            "BucketName": {
                "Type": "AWS::S3::Bucket"
            }
        }
    });

    let references = find_all_references(&template);
    let lambda_refs = &references["Lambda"];

    // MyBucket should be detected (from Ref)
    assert!(lambda_refs.contains("MyBucket"));

    // BucketName should be detected (from Fn::Sub template string)
    assert!(lambda_refs.contains("BucketName"));

    // Prefix should NOT be detected (it's a local variable in the map)
    assert!(!lambda_refs.contains("Prefix"));
}

#[test]
fn test_find_all_references_sub_escape_syntax() {
    // ${!Literal} syntax escapes the variable, treating it as a literal string
    // These should NOT be detected as resource references
    let template = json!({
        "Resources": {
            "Lambda": {
                "Type": "AWS::Lambda::Function",
                "Properties": {
                    "FunctionName": {
                        "Fn::Sub": "${!Literal}-${MyBucket}"
                    }
                }
            },
            "MyBucket": {
                "Type": "AWS::S3::Bucket"
            },
            "Literal": {
                "Type": "AWS::S3::Bucket"
            }
        }
    });

    let references = find_all_references(&template);
    let lambda_refs = &references["Lambda"];

    // MyBucket should be detected
    assert!(lambda_refs.contains("MyBucket"));

    // Literal should NOT be detected (it's escaped with ${!...})
    assert!(!lambda_refs.contains("Literal"));
}

#[test]
fn test_find_all_references_without_resources() {
    assert!(find_all_references(&json!({})).is_empty());
    assert!(find_all_references(&json!({"Resources": []})).is_empty());
}

#[test]
fn test_find_all_references_unusual_shapes() {
    let template = json!({
        "Resources": {
            "R": {
                "Type": "AWS::S3::Bucket",
                "DependsOn": ["Dep", 1],
                "Properties": {
                    "RefNumber": {"Ref": 1},
                    "GetAttString": {"Fn::GetAtt": "Str.Arn"},
                    "GetAttNumber": {"Fn::GetAtt": 1},
                    "GetAttEmpty": {"Fn::GetAtt": []},
                    "SubNumber": {"Fn::Sub": 1},
                    "SubFirstNotString": {"Fn::Sub": [1, {}]},
                    "SubWithoutMap": {"Fn::Sub": ["${Plain}"]},
                    "SubMapNotObject": {"Fn::Sub": ["${Listed}", []]},
                    "SubOddText": {"Fn::Sub": "$ $x ${} ${!Open"},
                    "SubUnclosed": {"Fn::Sub": "${Unclosed"},
                    "SubUnclosedAttr": {"Fn::Sub": "${Attr.Arn"}
                }
            },
            "S": {"Type": "AWS::SQS::Queue", "DependsOn": 1}
        }
    });

    let references = find_all_references(&template);

    let mut found: Vec<&String> = references["R"].iter().collect();
    found.sort();
    assert_eq!(found, ["Attr", "Dep", "Listed", "Plain", "Str", "Unclosed"]);
    assert!(!references.contains_key("S"));
}

#[test]
fn test_update_template_references_unusual_shapes() {
    let template = json!({
        "Resources": {
            "R": {
                "Type": "AWS::S3::Bucket",
                "DependsOn": ["Other", 1, "Old"],
                "Properties": {
                    "RefNumber": {"Ref": 1},
                    "RefOther": {"Ref": "Other"},
                    "GetAttEmpty": {"Fn::GetAtt": []},
                    "GetAttFirstNotString": {"Fn::GetAtt": [1, "Arn"]},
                    "GetAttOther": {"Fn::GetAtt": ["Other", "Arn"]},
                    "GetAttNoDot": {"Fn::GetAtt": "Old"},
                    "GetAttOtherString": {"Fn::GetAtt": "Other.Arn"},
                    "GetAttNumber": {"Fn::GetAtt": 1},
                    "SubNumber": {"Fn::Sub": 1},
                    "SubEmpty": {"Fn::Sub": []},
                    "SubFirstNotString": {"Fn::Sub": [1]},
                    "SubMapNotObject": {"Fn::Sub": ["${Old}", []]}
                }
            },
            "S": {"Type": "AWS::SQS::Queue", "DependsOn": "Other"},
            "T": {"Type": "AWS::SQS::Queue", "DependsOn": 1}
        }
    });
    let mapping = HashMap::from([("Old".to_string(), "New".to_string())]);

    let result = update_template_references(template.clone(), &mapping);

    let props = &result["Resources"]["R"]["Properties"];
    let before = &template["Resources"]["R"]["Properties"];
    for key in [
        "RefNumber",
        "RefOther",
        "GetAttEmpty",
        "GetAttFirstNotString",
        "GetAttOther",
        "GetAttNoDot",
        "GetAttOtherString",
        "GetAttNumber",
        "SubNumber",
        "SubEmpty",
        "SubFirstNotString",
    ] {
        assert_eq!(props[key], before[key], "{key}");
    }
    assert_eq!(props["SubMapNotObject"], json!({"Fn::Sub": ["${New}", []]}));
    assert_eq!(
        result["Resources"]["R"]["DependsOn"],
        json!(["Other", 1, "New"])
    );
    assert_eq!(result["Resources"]["S"]["DependsOn"], "Other");
    assert_eq!(result["Resources"]["T"]["DependsOn"], 1);
}
