# cliff [![Build Status](https://travis-ci.com/meetup/cliff.svg?branch=master)](https://travis-ci.com/meetup/cliff)

> An AWS CloudFormation stack diff tool

## 🥰 features

* visible, predictable infrastructure deployment
* optimized for human DX

## 🤔 about

[CloudFormation](https://aws.amazon.com/cloudformation/) is an awesome fully managed infrastructure as code tool but applying these changes to your existing infrastructure can sometimes have suprising effects. Cliff can help with that.

Cliff is enhances CloudFormation by enabling you to see your changes will look like before you make them  💅 . You can think of cliff as the combination of create changeset + describe changeset + diff packaged into one tool. You _can_ do all of these things without cliff using the aws cli the result will come at the expense of learning to cross stitch and understanding nuances of CloudFormation detalis 🧶. Cliff does that stitching for you with a display intended for humans.

## 🤸 usage

Basic usage requires two things. The name of an existing stack and a path to a template

```sh
$ cliff \
	--stack-name your-cloud-formation-stack-name \
	path/to/template.yml
```

Many CloudFormation templates will employ parameterizatin for flexibility. In those cases you'll want to use the `--parameters` or `-p` option.

```sh
$ cliff \
	--stack-name your-cloud-formation-stack-name \
	--parameters "Foo=bar" "Baz=boom" "quux=quuz" \
	path/to/template.yml
```

### diffing

By default cliff will `diff --label -u` to compare local and remote templates. If you would like a fancier diff tool, cliff will use the value of 
an environment variable `CLIFF_DIFFER` instead. If you are a [VS Code](https://code.visualstudio.com/) user you may want to use `CLIFF_DIFFER="code --wait --diff"` or if you prefer [colordiff](https://www.colordiff.org/), you might use `CLIFF_DIFFER=colordiff`
